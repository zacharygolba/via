use hyper::server::conn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::io;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::{JoinError, JoinSet};
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::util::fmt_elapsed;
use crate::app::{App, AppService};
use crate::error::DynError;
use crate::server::stream::IoStream;

/// The amount of time in seconds we'll wait before garbage collecting
/// connection tasks that finished.
///
const IDLE_TIMEOUT: Duration = Duration::from_secs(15);

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
            // The server probably won't shutdown in the next 15 seconds.
            biased;
            // A connection was accepted or IDLE_TIMEOUT expired.
            result = time::timeout(IDLE_TIMEOUT, listener.accept()) => {
                match result {
                    Ok(Ok(accepted)) => accepted,
                    Ok(Err(error)) => {
                        eprintln!("error(listener): {}", error);
                        drop(permit);
                        continue;
                    }
                    Err(_) => {
                        let before = connections.len();

                        if before > 0 {
                            let now = Instant::now();

                            if cfg!(debug_assertions) {
                                eprintln!("server is idle");
                                eprintln!("  joining connection tasks that finished");
                                eprintln!("    {} total tasks", before);
                            }

                            while let Some(result) = connections.try_join_next() {
                                joined_connection(&result);
                                if now.elapsed().as_micros() >= 1_000 {
                                    break;
                                }
                            }

                            if cfg!(debug_assertions) {
                                eprintln!(
                                    "    joined {} tasks in {}",
                                    before - connections.len(),
                                    fmt_elapsed(now.elapsed())
                                );
                            }
                        }

                        drop(permit);
                        continue;
                    }
                }
            }
            // A shutdown signal was received.
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

            // Create an AppService to serve the connection.
            let service = AppService::new(Arc::clone(&app), max_body_size);

            async move {
                let accepted = match acceptor.accept(stream).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        drop(permit);
                        return Err(error.into());
                    }
                };

                // Create a new HTTP/2 connection.
                #[cfg(feature = "http2")]
                let connection = conn::http2::Builder::new(TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .serve_connection(IoStream::new(accepted), service);

                // Create a new HTTP/1.1 connection.
                #[cfg(all(feature = "http1", not(feature = "http2")))]
                let connection = conn::http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(IoStream::new(accepted), service)
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

                // Explicitly drop the semaphore permit.
                drop(permit);

                result
            }
        });

        if semaphore.available_permits() == 0 {
            let now = Instant::now();

            if cfg!(debug_assertions) {
                eprintln!("server at capacity");
                eprintln!("  a single connection task will be joined");
            }

            if let Some(result) = connections.join_next().await {
                joined_connection(&result);
            }

            if cfg!(debug_assertions) {
                eprintln!("  task joined in {}", fmt_elapsed(now.elapsed()));
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

fn joined_connection(result: &Result<Result<(), DynError>, JoinError>) {
    match result {
        // An error occurred that originates from hyper or tokio.
        Ok(Err(error)) => {
            if let Some(e) = error.downcast_ref::<hyper::Error>() {
                let is_disconnect = e.is_canceled()
                    || e.is_incomplete_message()
                    || e.source().is_some_and(|source| {
                        source
                            .downcast_ref::<io::Error>()
                            .is_some_and(|e| e.kind() == io::ErrorKind::NotConnected)
                    });

                if cfg!(debug_assertions) {
                    if is_disconnect {
                        // trace!();
                    } else {
                        eprintln!("error(http): {}", e);
                    }
                }
            } else {
                eprintln!("error(other): {}", error);
            }
        }
        // The connection was cancelled or the panicked.
        Err(error) if error.is_panic() => {
            // Placeholder for tracing...
            eprintln!("panic: {}", error);
        }
        _ => {}
    }
}

async fn drain_connections(connections: &mut JoinSet<Result<(), DynError>>) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(result) = connections.join_next().await {
        joined_connection(&result);
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<bool>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(false)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}
