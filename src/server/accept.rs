use hyper::server::conn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::{signal, time};

use super::acceptor::Acceptor;
use super::conn::JoinQueue;
use crate::app::AppService;
use crate::error::DynError;

macro_rules! log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*)
        }
    };
}

pub async fn accept<A, T>(listener: &TcpListener, acceptor: &A, service: &AppService<T>) -> ExitCode
where
    A: Acceptor + Clone + Send + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(service.max_connections()));

    let mut ctrl_c = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinQueue::new();

    // Start accepting incoming connections.
    let exit_code = loop {
        // Acquire a permit from the semaphore.
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(acquired) => acquired,
            Err(_) => break ExitCode::from(1),
        };

        // Accept the next stream from the tcp listener.
        let (tcp_stream, _) = tokio::select! {
            joined = connections.join_next(), if !connections.is_empty() => {
                if let Some(result) = joined {
                    if let Err(error) = result.as_ref() {
                        handle_error(error);
                    }
                }
                drop(permit);
                continue;
            }
            result = listener.accept() => match result {
                Ok(accepted) => accepted,
                Err(error) => {
                    log!("error(listener): {}", error);
                    drop(permit);
                    continue;
                }
            },
            Ok(()) = ctrl_c.changed() => {
                drop(permit);
                break match *ctrl_c.borrow_and_update() {
                    Some(false) => ExitCode::from(0),
                    Some(true) | None => ExitCode::from(1),
                }
            }
        };

        connections.spawn({
            let acceptor = acceptor.clone();
            let service = service.clone();
            let mut ctrl_c = ctrl_c.clone();

            async move {
                let tls_stream = match acceptor.accept(tcp_stream).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        drop(permit);
                        return Err(error.into());
                    }
                };

                // Create a new HTTP/2 connection.
                #[cfg(feature = "http2")]
                let connection = conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .serve_connection(TokioIo::new(tls_stream), service);

                // Create a new HTTP/1.1 connection.
                #[cfg(all(feature = "http1", not(feature = "http2")))]
                let connection = conn::http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(TokioIo::new(tls_stream), service)
                    .with_upgrades();

                tokio::pin!(connection);

                // Serve the connection.
                let result = tokio::select! {
                    done = connection.as_mut() => done,
                    _ = ctrl_c.changed() => {
                        connection.as_mut().graceful_shutdown();
                        connection.as_mut().await
                    }
                };

                drop(permit);

                result.map_err(|e| e.into())
            }
        });
    };

    let shutdown = time::timeout(
        service.shutdown_timeout(),
        drain_connections(&mut connections),
    );

    if shutdown.await.is_ok() {
        exit_code
    } else {
        1.into()
    }
}

fn handle_error(error: &DynError) {
    let hyper_error = match error.downcast_ref::<hyper::Error>() {
        Some(error) => error,
        None => {
            log!("error(other): {}", error);
            return;
        }
    };

    if hyper_error.is_canceled()
        || hyper_error.is_incomplete_message()
        || hyper_error.source().is_some_and(|source| {
            source
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotConnected)
        })
    {
        // Disconnected.
    } else {
        log!("error(http): {}", hyper_error);
    }
}

async fn drain_connections(connections: &mut JoinQueue) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        if let Err(error) = result.as_ref() {
            handle_error(error);
        }
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<bool>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(false)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}
