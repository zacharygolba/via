use hyper::server::conn;
use hyper_util::rt::TokioTimer;
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
use crate::app::{App, AppService};
use crate::error::DynError;
use crate::server::stream::IoStream;

/// The maximum amount of connections that can be join at time while the server
/// is running.
///
const CONNECTION_JOIN_LIMIT: usize = 3;

macro_rules! joined {
    ($result:expr) => { joined!($result ; else let Err(error) {}) };
    ($result:expr ; else let Err($error:ident) $else:expr) => {
        match $result {
            // Succussfully joined the connection.
            Ok(Ok(_)) => {}
            // The connection was cancelled or the panicked.
            Err(error) => {
                if error.is_panic() {
                    // Placeholder for tracing...
                    if cfg!(debug_assertions) {
                        eprintln!("error: {}", error);
                    }
                }
            }
            // An error occurred that originates from hyper or tokio.
            Ok(Err($error)) => {
                // Placeholder for tracing...
                if cfg!(debug_assertions) {
                    eprintln!("error: {}", $error);
                }
                $else
            }
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

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::<Result<(), DynError>>::new();

    // Wrap app in an arc so it can be cloned into the connection task.
    let app = Arc::new(app);

    // Start accepting incoming connections.
    let exit_code = 'accept: loop {
        // Acquire a permit from the semaphore.
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(acquired) => acquired,
            Err(_) => break 1.into(),
        };

        // Accept the next stream from the tcp listener.
        let (stream, _addr) = tokio::select! {
            biased;
            result = listener.accept() => match result {
                Ok(accepted) => accepted,
                Err(error) => {
                    eprintln!("error(listener): {}", error);
                    drop(permit);
                    continue;
                }
            },
            _ = shutdown_rx.changed() => {
                drop(permit);
                break 'accept match *shutdown_rx.borrow_and_update() {
                    Some(false) => 0.into(),
                    Some(true) | None => 1.into(),
                }
            }
        };

        // Spawn a task to serve the connection.
        connections.spawn({
            // Clone acceptor so negotiation can happen in the connection task.
            let mut acceptor = acceptor.clone();

            // Clone the watch receiver so we can shutdown the connection if a
            // ctrl+c signal is sent to the process.
            let mut shutdown_rx = shutdown_rx.clone();

            // Clone the arc pointer to app so it can be moved into the task.
            let app = Arc::clone(&app);

            async move {
                let result = match acceptor.accept(stream).await {
                    Err(error) => Err(error.into()),
                    Ok(accepted) => {
                        // Create a new HTTP/2 connection.
                        #[cfg(feature = "http2")]
                        let connection = conn::http2::Builder::new(TokioExecutor::new())
                            .timer(TokioTimer::new())
                            .serve_connection(
                                IoStream::new(accepted),
                                AppService::new(app, max_body_size),
                            );

                        // Create a new HTTP/1.1 connection.
                        #[cfg(all(feature = "http1", not(feature = "http2")))]
                        let connection = conn::http1::Builder::new()
                            .timer(TokioTimer::new())
                            .serve_connection(
                                IoStream::new(accepted),
                                AppService::new(app, max_body_size),
                            )
                            .with_upgrades();

                        tokio::pin!(connection);

                        // Serve the connection.
                        tokio::select! {
                            result = &mut connection => result.map_err(|e| e.into()),
                            _ = shutdown_rx.changed() => {
                                connection.as_mut().graceful_shutdown();
                                connection.await.map_err(|e| e.into())
                            }
                        }
                    }
                };

                drop(permit);
                result
            }
        });

        for _ in 0..CONNECTION_JOIN_LIMIT {
            match connections.try_join_next() {
                Some(result) => joined!(&result),
                None => break,
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

async fn drain_connections(connections: &mut JoinSet<Result<(), DynError>>) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        joined!(&result);
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<bool>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(false)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}
