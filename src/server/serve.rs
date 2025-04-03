use hyper::server::conn;
use hyper_util::rt::TokioTimer;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::{JoinError, JoinSet};
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use crate::app::{App, AppService};
use crate::error::DynError;
use crate::server::io_stream::IoStream;

pub async fn serve<A, T>(
    listener: TcpListener,
    acceptor: A,
    app: App<T>,
    max_body_size: usize,
    max_connections: usize,
    shutdown_timeout: Duration,
) -> Result<ExitCode, DynError>
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    // Wrap app in an arc so it can be cloned into the connection task.
    let app = Arc::new(app);

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Start accepting incoming connections.
    let exit_code = 'accept: loop {
        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_owned().await?;

        // Wait for something interesting to happen.
        let tcp_stream = loop {
            tokio::select! {
                biased;

                // A new connection is ready to be accepted.
                result = listener.accept() => match result {
                    // Accept the stream from the acceptor.
                    Ok((stream, _addr)) => break stream,
                    Err(error) => {
                        // Placeholder for tracing...
                        if cfg!(debug_assertions) {
                            eprintln!("error(listener): {}", error);
                        }
                    }
                },

                // We have idle time. Join any inflight connections that may
                // have finished.
                first = connections.join_next(), if !connections.is_empty() => {
                    if let Some(result) = first {
                        handle_connection_result(result);
                    }

                    while let Some(result) = connections.try_join_next() {
                        handle_connection_result(result);
                    }
                }

                // A graceful shutdown was requested.
                _ = shutdown_rx.changed() => {
                    // Break out of the accept loop with the corrosponding exit code.
                    break 'accept match *shutdown_rx.borrow_and_update() {
                        Some(false) => ExitCode::from(0),
                        Some(true) | None => ExitCode::from(1),
                    }
                }
            }
        };

        // Create an AppService to serve the connection.
        let service = AppService::new(Arc::clone(&app), max_body_size);

        // Clone acceptor so negotiation can happen in the connection task.
        let mut acceptor = acceptor.clone();

        // Clone the watch receiver so we can shutdown the connection if a
        // ctrl+c signal is sent to the process.
        let mut shutdown_rx = shutdown_rx.clone();

        // Spawn a task to serve the connection.
        connections.spawn(async move {
            let result = match acceptor.accept(tcp_stream).await {
                Err(error) => Err(error.into()),
                Ok(accepted) => {
                    // Create a new HTTP/2 connection.
                    #[cfg(feature = "http2")]
                    let mut connection = conn::http2::Builder::new(TokioExecutor::new())
                        .timer(TokioTimer::new())
                        .serve_connection(IoStream::new(accepted), service);

                    // Create a new HTTP/1.1 connection.
                    #[cfg(all(feature = "http1", not(feature = "http2")))]
                    let mut connection = conn::http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(IoStream::new(accepted), service)
                        .with_upgrades();

                    // Pin the connection on the stack so it can be polled.
                    let mut connection = Pin::new(&mut connection);

                    // Serve the connection.
                    tokio::select! {
                        biased;
                        result = &mut connection => result.map_err(|e| e.into()),
                        _ = shutdown_rx.changed() => {
                            connection.as_mut().graceful_shutdown();
                            connection.as_mut().await.map_err(|e| e.into())
                        }
                    }
                }
            };

            // Explicitly drop the semaphore permit.
            drop(permit);

            result
        });
    };

    tokio::select! {
        // Wait for inflight connection to close within the configured timeout.
        _ = shutdown(&mut connections) => Ok(exit_code),

        // Otherwise, return an error.
        _ = time::sleep(shutdown_timeout) => {
            Err("server exited before all connections were closed".into())
        }
    }
}

fn handle_connection_result(result: Result<Result<(), DynError>, JoinError>) {
    match result {
        Err(error) if error.is_panic() => {
            // Placeholder for tracing...
            if cfg!(debug_assertions) {
                eprintln!("error(connection): {}", error);
            }
        }
        Ok(Err(error)) => {
            // Placeholder for tracing...
            if cfg!(debug_assertions) {
                eprintln!("error(connection): {}", error);
            }
        }
        _ => {}
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<bool>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(false)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}

async fn shutdown(connections: &mut JoinSet<Result<(), DynError>>) {
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            let _ = &error; // Placeholder for tracing...
        }
    }
}
