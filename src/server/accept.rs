use hyper::server::conn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};
use tokio::task::JoinSet;
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::io::IoWithPermit;
use super::server::ServerConfig;
use crate::app::AppService;
use crate::error::ServerError;

macro_rules! log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*)
        }
    };
}

#[inline(never)]
pub async fn accept<A, T>(
    listener: TcpListener,
    acceptor: A,
    app_service: AppService<T>,
    server_config: ServerConfig,
) -> ExitCode
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(server_config.max_connections));

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // A JoinSet to track inflight connections. We use this to drain connection
    // tasks before returning the exit code.
    let mut connections = JoinSet::new();

    // Start accepting incoming connections.
    let exit_code = loop {
        // Acquire a permit from the semaphore.
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(acquired) => acquired,
            Err(_) => break ExitCode::FAILURE,
        };

        let (tcp_stream, _) = tokio::select! {
            // A new TCP stream was accepted from the listener.
            result = listener.accept() => match result {
                Ok(accepted) => accepted,
                Err(error) => {
                    log!("error(accept): {}", error);
                    continue;
                }
            },
            // The process received a graceful shutdown signal.
            _ = shutdown_rx.changed() => {
                break shutdown_rx.borrow_and_update().unwrap_or(ExitCode::FAILURE);
            }
        };

        let mut acceptor = acceptor.clone();
        let app_service = app_service.clone();
        let mut shutdown_rx = shutdown_rx.clone();

        // Spawn a task to serve the connection.
        connections.spawn(async move {
            // Accept the TCP stream from the acceptor.
            let io = match acceptor.accept(tcp_stream).await {
                Ok(accepted) => IoWithPermit::new(permit, accepted),
                Err(error) => {
                    handle_error(ServerError::Io(&error));
                    return;
                }
            };

            // Create a new HTTP/2 connection.
            #[cfg(feature = "http2")]
            let mut connection = Box::pin(
                conn::http2::Builder::new(TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .serve_connection(io, app_service),
            );

            // Create a new HTTP/1.1 connection.
            #[cfg(all(feature = "http1", not(feature = "http2")))]
            let mut connection = Box::pin(
                conn::http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, app_service)
                    .with_upgrades(),
            );

            // Serve the connection.
            let result = tokio::select! {
                result = connection.as_mut() => result,
                _ = shutdown_rx.changed() => {
                    connection.as_mut().graceful_shutdown();
                    connection.as_mut().await
                }
            };

            if let Err(error) = &result {
                handle_error(ServerError::Hyper(error));
            }
        });

        // Reap up to 2 finished connection tasks. This keeps the length of
        // connections roughly around `server_config.max_connections` in the
        // worst case scenario.
        'try_join: for _ in 0..2 {
            match connections.try_join_next() {
                Some(Err(error)) => handle_error(ServerError::Join(&error)),
                Some(Ok(_)) => {}
                None => break 'try_join,
            }
        }
    };

    let drain_all = time::timeout(
        Duration::from_secs(server_config.shutdown_timeout),
        drain_connections(&mut connections),
    );

    if drain_all.await.is_ok() {
        exit_code
    } else {
        ExitCode::FAILURE
    }
}

fn handle_error(error: ServerError) {
    match error {
        ServerError::Io(io_error) => log!("error(task): {}", io_error),
        ServerError::Join(join_error) => {
            if join_error.is_panic() {
                log!("panic(task): {}", join_error);
            }
        }
        ServerError::Hyper(hyper_error) => {
            let was_disconnect = hyper_error.is_canceled()
                || hyper_error.is_incomplete_message()
                || hyper_error.source().is_some_and(|source| {
                    source
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::NotConnected)
                });

            if !was_disconnect {
                log!("error(task): {}", hyper_error);
            }
        }
    }
}

async fn drain_connections(connections: &mut JoinSet<()>) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        if let Err(error) = &result {
            handle_error(ServerError::Join(error));
        }
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<ExitCode>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(ExitCode::SUCCESS)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}
