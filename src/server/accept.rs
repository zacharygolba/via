use hyper::server::conn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::{JoinError, JoinSet};
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::server::ServerContext;
use crate::error::DynError;

type TaskResult = Result<(), DynError>;

macro_rules! log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*)
        }
    };
}

pub async fn accept<A, T>(
    listener: TcpListener,
    acceptor: A,
    context: &ServerContext<T>,
) -> ExitCode
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(context.max_connections()));

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // Start accepting incoming connections.
    let exit_code = 'accept: loop {
        // Acquire a permit from the semaphore.
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(acquired) => acquired,
            Err(_) => break 1.into(),
        };

        let service = context.make_service();

        // Accept the next stream from the tcp listener.
        let tcp_stream = {
            let mut will_join_next = !connections.is_empty();
            let accepted = loop {
                tokio::select! {
                    biased;

                    joined = connections.join_next(), if will_join_next => {
                        joined.inspect(handle_joined);
                        will_join_next = false;
                        continue;
                    }

                    result = listener.accept() => {
                        break result;
                    }

                    Ok(()) = shutdown_rx.changed() => {
                        break 'accept match *shutdown_rx.borrow_and_update() {
                            Some(false) => 0.into(),
                            Some(true) | None => 1.into(),
                        }
                    }
                };
            };

            match accepted {
                Ok((stream, _)) => stream,
                Err(error) => {
                    drop(permit);
                    eprintln!("error(listener): {}", error);
                    continue;
                }
            }
        };

        connections.spawn({
            let mut acceptor = acceptor.clone();
            let mut shutdown_rx = shutdown_rx.clone();

            async move {
                let io = match acceptor.accept(tcp_stream).await {
                    Ok(stream) => TokioIo::new(stream),
                    Err(error) => {
                        drop(permit);
                        return Err(error.into());
                    }
                };

                // Create a new HTTP/2 connection.
                #[cfg(feature = "http2")]
                let connection = conn::http2::Builder::new(TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .serve_connection(io, service);

                // Create a new HTTP/1.1 connection.
                #[cfg(all(feature = "http1", not(feature = "http2")))]
                let connection = conn::http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service)
                    .with_upgrades();

                tokio::pin!(connection);

                // Serve the connection.
                let result = tokio::select! {
                    served = &mut connection => served.map_err(|e| e.into()),
                    _ = shutdown_rx.changed() => {
                        connection.as_mut().graceful_shutdown();
                        connection.await.map_err(|e| e.into())
                    }
                };

                drop(permit);

                result
            }
        });

        if let Some(result) = connections.try_join_next() {
            handle_joined(&result);
        }
    };

    let shutdown = time::timeout(
        context.shutdown_timeout(),
        drain_connections(&mut connections),
    );

    if shutdown.await.is_ok() {
        exit_code
    } else {
        1.into()
    }
}

fn handle_joined(result: &Result<TaskResult, JoinError>) {
    match result {
        // An error occurred that originates from hyper or tokio.
        Ok(Err(error)) => {
            if let Some(e) = error.downcast_ref::<hyper::Error>() {
                let is_disconnect = e.is_canceled()
                    || e.is_incomplete_message()
                    || e.source().is_some_and(|source| {
                        source
                            .downcast_ref::<std::io::Error>()
                            .is_some_and(|e| e.kind() == std::io::ErrorKind::NotConnected)
                    });

                if is_disconnect {
                    // trace!();
                } else {
                    log!("error(http): {}", e);
                }
            } else {
                log!("error(other): {}", error);
            }
        }
        // The connection task panicked.
        Err(error) if error.is_panic() => panic!("{}", error),
        Ok(_) | Err(_) => {}
    }
}

async fn drain_connections(connections: &mut JoinSet<TaskResult>) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(connection) = connections.join_next().await {
        handle_joined(&connection);
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<bool>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(false)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}
