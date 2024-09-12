use hyper::server::conn::http1;
use hyper_util::rt::TokioTimer;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;
use tokio::{signal, task, time};

use super::io_stream::IoStream;
use super::service::Service;
use crate::router::Router;
use crate::Error;

pub async fn serve<State>(
    state: Arc<State>,
    router: Arc<Router<State>>,
    listener: TcpListener,
    max_connections: usize,
    shutdown_timeout: Duration,
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
{
    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    let (shutdown_task, shutdown_rx) = {
        // Create a watch channel to notify the connections to initiate a
        // graceful shutdown process when the `ctrl_c` future resolves.
        let (tx, rx) = watch::channel(false);

        // Spawn a task to wait for a "Ctrl-C" signal to be sent to the process.
        let task = task::spawn(async move {
            match signal::ctrl_c().await {
                Ok(_) => tx.send(true).map_err(|_| {
                    let message = "unable to notify connections to shutdown.";
                    Error::new(message.to_string())
                }),
                Err(error) => {
                    if cfg!(debug_assertions) {
                        eprintln!("unable to register the 'Ctrl-C' signal.");
                    }

                    Err(error.into())
                }
            }
        });

        (task, rx)
    };

    loop {
        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_many_owned(2).await?;

        // Create a new Service. We'll move this into the task when a new
        // connection is accepted.
        let service = Service::new(Arc::clone(&router), Arc::clone(&state));

        // Clone the watch channel so that we can notify the connection when
        // initiate a graceful shutdown process before the server exits.
        let mut shutdown_rx = shutdown_rx.clone();

        tokio::select! {
            // Wait for a new connection to be accepted.
            result = listener.accept() => {
                let (stream, _addr) = match result {
                    Ok(accepted) => accepted,
                    Err(_) => {
                        // Placeholder for tracing...
                        continue;
                    }
                };

                // Spawn a task to serve the connection.
                connections.spawn(async move {
                    // Create a new connection for the configured HTTP version.
                    // For now we only support HTTP/1.1. This will be expanded to
                    // support HTTP/2 in the future.
                    let mut connection = http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(IoStream::new(stream), service)
                        .with_upgrades();

                    // Poll the connection until it is closed or a graceful
                    // shutdown process is initiated.
                    let result = tokio::select! {
                        // Pin the connection on the stack so it can be polled
                        // to completion. This is the typical path that the code
                        // should take while the server is running.
                        result = Pin::new(&mut connection) => result,

                        // Otherwise, wait until `shutdown_rx` is notified that
                        // the server will shutdown and initiate a graceful
                        // shutdown process for the connection.
                        _ = shutdown_rx.changed() => {
                            let mut connection = Pin::new(&mut connection);

                            // Initiate the graceful shutdown process for the
                            // connection.
                            connection.as_mut().graceful_shutdown();

                            // Wait for the connection to close.
                            connection.await
                        }
                    };

                    // Release the permit back to the semaphore.
                    drop(permit);

                    if let Err(_) = result {
                        // Placeholder for tracing..
                    }
                });
            }

            // Otherwise, wait for a "Ctrl-C" signal to be sent to the process.
            _ = shutdown_rx.changed() => {
                // Break out of the loop to stop accepting new connections.
                break;
            }
        }

        // Remove any handles that may have finished.
        while let Some(result) = connections.try_join_next() {
            if let Err(error) = result {
                // Placeholder for tracing...

                if cfg!(debug_assertions) {
                    eprintln!("Error: {}", error);
                }
            }
        }
    }

    let shutdown_started_at = Instant::now();

    tokio::select! {
        // Wait for all inflight connection to finish. If all connections close
        // before the graceful shutdown timeout, return without an error. For
        // unix-based systems, this translates to a 0 exit code.
        _ = shutdown(connections) => {
            let elapsed_as_seconds = shutdown_started_at.elapsed().as_secs();
            let timeout_as_seconds = shutdown_timeout.as_secs();
            let remaining_timeout = timeout_as_seconds
                .checked_sub(elapsed_as_seconds)
                .map_or(Duration::from_secs(10), Duration::from_secs);

            // Wait for the shutdown task to complete before exiting the server.
            time::timeout(remaining_timeout, shutdown_task).await???;

            // The shutdown_task completed within the timeout.
            Ok(())
        }

        // Otherwise, return an error if we're unable to close all connections
        // before the graceful shutdown timeout, return an error. For unix-based
        // systems, this translates to a 1 exit code.
        _ = time::sleep(shutdown_timeout) => {
            Err(Error::new("server exited before all connections were closed.".to_string()))
        }
    }
}

async fn shutdown(connections: JoinSet<()>) -> Result<(), Error> {
    if cfg!(debug_assertions) {
        eprintln!(
            "waiting for {} inflight connection(s) to close...",
            connections.len()
        );
    }

    // Wait for all inflight connections to close.
    connections.join_all().await;

    Ok(())
}
