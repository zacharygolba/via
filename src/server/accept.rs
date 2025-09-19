use hyper::server::conn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tokio::task::JoinSet;
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::io::IoWithPermit;
use crate::app::{App, AppService};
use crate::error::ServerError;

macro_rules! accept_with_timeout {
    ($future:expr, $immediate:expr) => {
        time::timeout(
            // Timeout after 1 second or 1 day.
            Duration::from_secs(if $immediate { 1 } else { 60 * 60 * 24 }),
            $future,
        )
    };
}

macro_rules! joined {
    ($result:expr) => {
        match $result {
            Ok(Err(error)) => handle_error(&error),
            Err(error) => handle_error(&ServerError::Join(error)),
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

pub async fn accept<A, T>(
    listener: TcpListener,
    acceptor: A,
    app: App<T>,
    max_body_size: usize,
    max_connections: usize,
    shutdown_timeout: u64,
) -> ExitCode
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::<Result<(), ServerError>>::new();

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // We alternate between accepting new connections and joining finished
    // ones. This avoids starving connection tasks while keeping accept latency
    // low. In practice, this means we only call `join_next()` every other turn.
    let mut should_join_next = false;

    // Wrap app in an arc so it can be cloned into the connection task.
    let app = Arc::new(app);

    // Start accepting incoming connections.
    let exit_code = loop {
        // Acquire a permit from the semaphore.
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(acquired) => acquired,
            Err(_) => break ExitCode::FAILURE,
        };

        tokio::select! {
            // Poll the listener before anything else.
            biased;

            // Accept a connection from the TCP listener.
            result = accept_with_timeout!(listener.accept(), !connections.is_empty()) => {
                match result {
                    // Connection accepted.
                    Ok(Ok((tcp_stream, _))) => {
                        // Spawn a task to serve the connection.
                        connections.spawn(handle_connection(
                            permit,
                            tcp_stream,
                            acceptor.clone(),
                            shutdown_rx.clone(),
                            Arc::clone(&app),
                            max_body_size,
                        ));

                        // Flip the `should_join_next` bit.
                        should_join_next = !should_join_next;
                    }
                    // Error, burn the permit and try again.
                    Ok(Err(error)) => {
                        log!("error(accept): {}", error);
                    }
                    // Idle, set `should_join_next` and try again.
                    Err(_) => {
                        should_join_next = true;
                    }
                }
            }

            // Maybe try to join a connection while we wait.
            Some(result) = connections.join_next(), if should_join_next => {
                should_join_next = false;
                joined!(result);
            }

            // Break if we receive a graceful shutdown signal.
            _shutdown_signal = shutdown_rx.changed() => {
                break shutdown_rx.borrow_and_update().unwrap_or(ExitCode::FAILURE);
            }
        }

        // Try to join a connection task opportunistically.
        if let Some(result) = connections.try_join_next() {
            joined!(result);
        }
    };

    let drain_all = time::timeout(
        Duration::from_secs(shutdown_timeout),
        drain_connections(&mut connections),
    );

    if drain_all.await.is_ok() {
        exit_code
    } else {
        ExitCode::FAILURE
    }
}

fn handle_error(error: &ServerError) {
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

async fn handle_connection<A, T>(
    permit: OwnedSemaphorePermit,
    tcp_stream: TcpStream,
    mut acceptor: A,
    mut shutdown_rx: watch::Receiver<Option<ExitCode>>,
    app: Arc<App<T>>,
    max_body_size: usize,
) -> Result<(), ServerError>
where
    A: Acceptor + 'static,
    T: Send + Sync,
{
    // Accept the TCP stream from the acceptor.
    let maybe_tls_stream = acceptor.accept(tcp_stream).await?;

    // Create a new HTTP/2 connection.
    #[cfg(feature = "http2")]
    let mut connection = Box::pin(
        conn::http2::Builder::new(TokioExecutor::new())
            .timer(TokioTimer::new())
            .serve_connection(
                IoWithPermit::new(permit, maybe_tls_stream),
                AppService::new(app, max_body_size),
            ),
    );

    // Create a new HTTP/1.1 connection.
    #[cfg(all(feature = "http1", not(feature = "http2")))]
    let mut connection = Box::pin(
        conn::http1::Builder::new()
            .timer(TokioTimer::new())
            .serve_connection(
                IoWithPermit::new(permit, maybe_tls_stream),
                AppService::new(app, max_body_size),
            )
            .with_upgrades(),
    );

    // Serve the connection.
    tokio::select! {
        result = connection.as_mut() => result.map_err(ServerError::from),
        _ = shutdown_rx.changed() => {
            connection.as_mut().graceful_shutdown();
            connection.as_mut().await.map_err(ServerError::from)
        }
    }
}

async fn drain_connections(connections: &mut JoinSet<Result<(), ServerError>>) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        joined!(result);
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<ExitCode>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(ExitCode::SUCCESS)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}
