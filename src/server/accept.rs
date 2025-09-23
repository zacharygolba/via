use hyper::server::conn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::mem;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};
use tokio::task::{JoinSet, coop};
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
    acceptor: Arc<A>,
    service: AppService<T>,
    config: ServerConfig,
) -> ExitCode
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently.
    //
    // If the maximum number of connections is reached, we'll wait until
    // `config.accept_timeout` before resetting the connection.
    let semaphore = Arc::new(Semaphore::new(config.max_connections));

    // Notify the accept loop and connection tasks to initiate a graceful
    // shutdown when a "ctrl-c" notification is sent to the process.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // A JoinSet to track and join active connections.
    let mut connections = JoinSet::new();

    let ServerConfig {
        accept_timeout,
        shutdown_timeout,
        ..
    } = config;

    // Start accepting incoming connections.
    let exit_code = loop {
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

        // Acquire a permit from the semaphore.
        let permit = match semaphore.clone().try_acquire_owned() {
            // We were able to acquire a permit without blocking accept.
            Ok(acquired) => acquired,

            // The server is at capacity. Try to acquire a permit with the
            // configured timeout.
            Err(_) => {
                let acquire = semaphore.clone().acquire_owned();

                match time::timeout(accept_timeout, acquire).await {
                    // Permit acquired!
                    Ok(Ok(acquired)) => acquired,

                    // The semaphore was dropped. Likely unreachable.
                    Ok(Err(_)) => break ExitCode::FAILURE,

                    // The server is still at capacity. Reset the connection.
                    Err(_) => continue,
                }
            }
        };

        let acceptor = acceptor.clone();
        let service = service.clone();
        let mut shutdown_rx = shutdown_rx.clone();

        // Spawn a task to serve the connection.
        connections.spawn(async move {
            // Accept the TCP stream from the acceptor.
            let io = match acceptor.accept(tcp_stream).await {
                Ok(accepted) => IoWithPermit::new(permit, accepted),
                Err(error) => return Err(ServerError::Io(error)),
            };

            // Create a new HTTP/2 connection.
            #[cfg(feature = "http2")]
            let mut connection = Box::pin(
                conn::http2::Builder::new(TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .serve_connection(io, service),
            );

            // Create a new HTTP/1.1 connection.
            #[cfg(all(feature = "http1", not(feature = "http2")))]
            let mut connection = Box::pin(
                conn::http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service)
                    .with_upgrades(),
            );

            // Serve the connection.
            tokio::select! {
                result = connection.as_mut() => result.map_err(ServerError::Hyper),
                _ = shutdown_rx.changed() => {
                    connection.as_mut().graceful_shutdown();
                    connection.as_mut().await.map_err(ServerError::Hyper)
                }
            }
        });

        if connections.len() >= 1024 {
            let batch = mem::take(&mut connections);
            tokio::spawn(drain_connections(false, batch));
        }
    };

    // Try to drain each inflight connection before `config.shutdown_timeout`.
    let drain = drain_connections(true, connections);

    match time::timeout(shutdown_timeout, drain).await {
        Ok(_) => exit_code,
        Err(_) => ExitCode::FAILURE,
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

async fn drain_connections(immediate: bool, mut connections: JoinSet<Result<(), ServerError>>) {
    if cfg!(debug_assertions) {
        println!("joining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        match result {
            Ok(Err(error)) => handle_error(error),
            Err(error) => handle_error(ServerError::Join(error)),
            _ => {}
        }

        if !immediate {
            coop::consume_budget().await;
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
