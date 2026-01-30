use hyper::server::conn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::io;
use std::mem;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};
use tokio::task::{JoinSet, coop};
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::io::IoWithPermit;
use super::server::ServerConfig;
use super::tls::Acceptor;
use crate::app::AppService;
use crate::error::ServerError;

macro_rules! joined {
    ($result:expr) => {
        match $result {
            Ok(Err(error)) => handle_error(&error),
            Err(error) => log!("error(join): {}", error),
            _ => {}
        }
    };
}

macro_rules! log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*)
        }
    };
}

#[inline(never)]
pub async fn accept<App, TlsAcceptor>(
    config: ServerConfig,
    acceptor: TlsAcceptor,
    service: AppService<App>,
    listener: TcpListener,
) -> ExitCode
where
    App: Send + Sync + 'static,
    ServerError: From<TlsAcceptor::Error>,
    TlsAcceptor: Acceptor,
    TlsAcceptor::Io: Send + Unpin + 'static,
{
    let tls_timeout_in_seconds = config.tls_handshake_timeout;

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently.
    let semaphore = Arc::new(Semaphore::new(config.max_connections));

    // Notify the accept loop and connection tasks to initiate a graceful
    // shutdown when a "ctrl-c" notification is sent to the process.
    let mut watcher = wait_for_ctrl_c();

    // A JoinSet to track and join active connections.
    let mut connections = JoinSet::new();

    // Start accepting incoming connections.
    let exit_code = loop {
        let (io, _) = tokio::select! {
            // A new TCP stream was accepted from the listener.
            result = listener.accept() => match result {
                Err(error) if is_fatal(&error) => return ExitCode::FAILURE,
                Ok(accepted) => accepted,
                Err(error) => {
                    log!("error(accept): {}", error);
                    continue;
                }
            },

            // The process received a graceful shutdown signal.
            _ = watcher.changed() => {
                break Option::unwrap_or(*watcher.borrow_and_update(), ExitCode::FAILURE);
            }
        };

        // Permit acquired. Proceed with serving the connection.
        let Ok(permit) = semaphore.clone().try_acquire_owned() else {
            // The server is at capacity. Close the connection. Upstream load
            // balancers take this as a hint that it is time to try another
            // node.
            continue;
        };

        let handshake = acceptor.accept(io);
        let service = service.clone();
        let mut rx = watcher.clone();

        // Spawn a task to serve the connection.
        connections.spawn(async move {
            let io = if let Some(duration) = tls_timeout_in_seconds {
                let Ok(result) = time::timeout(duration, handshake).await else {
                    return Err(ServerError::handshake_timeout());
                };
                result?
            } else {
                handshake.await?
            };

            #[cfg(feature = "http2")]
            let connection = conn::http2::Builder::new(TokioExecutor::new())
                .timer(TokioTimer::new())
                .serve_connection(IoWithPermit::new(io, permit), service);

            #[cfg(all(feature = "http1", not(feature = "http2")))]
            let connection = conn::http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(IoWithPermit::new(io, permit), service)
                .with_upgrades();

            tokio::pin!(connection);

            // Serve the connection.
            tokio::select! {
                result = &mut connection => Ok(result?),
                _ = rx.changed() => {
                    connection.as_mut().graceful_shutdown();
                    Ok((&mut connection).await?)
                }
            }
        });

        if connections.len() >= 1024 {
            let batch = mem::take(&mut connections);
            tokio::spawn(drain_connections(false, batch));
        } else if let Some(result) = connections.try_join_next() {
            joined!(result);
        }
    };

    // Try to drain each inflight connection before `config.shutdown_timeout`.
    match time::timeout(
        config.shutdown_timeout,
        drain_connections(true, connections),
    )
    .await
    {
        Ok(_) => exit_code,
        Err(_) => ExitCode::FAILURE,
    }
}

async fn drain_connections(immediate: bool, mut connections: JoinSet<Result<(), ServerError>>) {
    if cfg!(debug_assertions) {
        println!("joining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        joined!(result);
        if !immediate {
            coop::consume_budget().await;
        }
    }
}

fn handle_error(error: &ServerError) {
    if let ServerError::Http(error) = error {
        if error.is_canceled()
            || error.is_incomplete_message()
            || error.source().is_some_and(|source| {
                source
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotConnected)
            })
        {
            log!("warn(disconnect): {}", error);
        } else {
            log!("error(http): {}", error);
        }
    } else {
        log!("error(task): {}", &error);
    }
}

#[cfg(unix)]
fn is_fatal(error: &io::Error) -> bool {
    if let std::io::ErrorKind::Other = error.kind() {
        matches!(error.raw_os_error(), Some(12 | 23 | 24))
    } else {
        false
    }
}

#[cfg(windows)]
fn is_fatal(error: &io::Error) -> bool {
    if let std::io::ErrorKind::Other = error.kind() {
        matches!(error.raw_os_error(), Some(10024 | 10055))
    } else {
        false
    }
}

fn wait_for_ctrl_c() -> watch::Receiver<Option<ExitCode>> {
    let (tx, rx) = watch::channel(None);

    tokio::spawn(async move {
        if signal::ctrl_c().await.is_err() {
            eprintln!("unable to register the 'ctrl-c' signal.");
        } else if tx.send(Some(ExitCode::SUCCESS)).is_err() {
            eprintln!("unable to notify connections to shutdown.");
        }
    });

    rx
}
