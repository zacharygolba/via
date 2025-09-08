use hyper::server::conn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::cell::Cell;
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, watch};
use tokio::task::{JoinError, JoinSet};
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use crate::app::{App, AppService};
use crate::error::ServerError;

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

    // A flag that tracks whether or not there are inflight connections that
    // can be joined.
    //
    // Using Cell<bool> makes reads and writes explicit and accidental movement
    // a compiler error. These are all things we want in such a hot path.
    let try_join = Cell::new(false);

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
            // Wait for the next stream from the tcp listener.
            accepted = listener.accept() => {
                match accepted {
                    Err(error) => log!("error(accept): {}", error),
                    Ok((stream, _)) => {
                        let future = handle_connection(
                            stream,
                            acceptor.clone(),
                            shutdown_rx.clone(),
                            Arc::clone(&app),
                            max_body_size,
                        );

                        // Spawn a task to serve the connection.
                        connections.spawn(async {
                            let _permit = permit; // RAII-guard
                            future.await
                        });

                        // Start joining connections again.
                        try_join.set(true);
                    }
                }
            },
            // Try to join an inflight connection while we wait.
            joined = connections.join_next(), if try_join.get() => {
                match &joined {
                    // Join error.
                    Some(Err(error)) => handle_error(error),
                    // Join success.
                    Some(Ok(_)) => {}
                    // The JoinSet is empty.
                    None => try_join.set(false),
                }
            }
            // Wait for a graceful shutdown signal.
            _exit = shutdown_rx.changed() => {
                break shutdown_rx.borrow_and_update().unwrap_or(ExitCode::FAILURE);
            }
        }
    };

    let drain_all = time::timeout(
        Duration::from_secs(shutdown_timeout),
        drain_connections(&mut connections),
    );

    if drain_all.await.is_ok() {
        exit_code
    } else {
        1.into()
    }
}

fn handle_error(error: &JoinError) {
    let hyper_error = error.source().and_then(|source| {
        let downcast = source.downcast_ref::<hyper::Error>()?;

        if downcast.is_canceled()
            || downcast.is_incomplete_message()
            || downcast.source().is_some_and(|source| {
                source
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotConnected)
            })
        {
            // Disconnected.
            None
        } else {
            Some(downcast)
        }
    });

    if let Some(log_as_http) = hyper_error {
        log!("error(http): {}", log_as_http);
    } else {
        log!("error(other): {}", error);
    }
}

async fn handle_connection<A, T>(
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
    let io_stream = TokioIo::new(acceptor.accept(tcp_stream).await?);
    let app_service = AppService::new(app, max_body_size);

    // Create a new HTTP/2 connection.
    #[cfg(feature = "http2")]
    let mut connection = Box::pin(
        conn::http2::Builder::new(TokioExecutor::new())
            .timer(TokioTimer::new())
            .serve_connection(io_stream, app_service),
    );

    // Create a new HTTP/1.1 connection.
    #[cfg(all(feature = "http1", not(feature = "http2")))]
    let mut connection = Box::pin(
        conn::http1::Builder::new()
            .timer(TokioTimer::new())
            .serve_connection(io_stream, app_service)
            .with_upgrades(),
    );

    // Serve the connection.
    tokio::select! {
        result = connection.as_mut() => result.map_err(|e| e.into()),
        _ = shutdown_rx.changed() => {
            connection.as_mut().graceful_shutdown();
            connection.await.map_err(|e| e.into())
        }
    }
}

async fn drain_connections(connections: &mut JoinSet<Result<(), ServerError>>) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        if let Err(error) = result.as_ref() {
            handle_error(error);
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
