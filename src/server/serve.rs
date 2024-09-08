use hyper::server::conn::http1;
use hyper_util::rt::TokioTimer;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;
use tokio::time;

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
    // Create a join set to track inflight connections. We'll use this to wait
    // for all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Get a Future that resolves when a "Ctrl-C" notification is sent to the
    // process. This is used to initiate a graceful shutdown process for
    // inflight connections.
    let mut ctrl_c = Box::pin(tokio::signal::ctrl_c());

    // Create a watch channel to notify the connections to initiate a graceful
    // shutdown when the `ctrl_c` future resolves.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    loop {
        tokio::select! {
            // Wait for a new connection to be accepted.
            result = accept(&listener, &semaphore) => {
                let (permit, (stream, _addr)) = match result {
                    Ok(accepted) => accepted,
                    Err(_) => {
                        //
                        // TODO:
                        //
                        // Include tracing information about why the connection
                        // could not be accepted.
                        //
                        continue;
                    }
                };

                // Wrap the TcpStream in a type that implements hyper's I/O traits.
                let io = IoStream::new(stream);

                // Create a new service to handle the connection.
                let service = {
                    let router = Arc::clone(&router);
                    let state = Arc::clone(&state);

                    Service::new(router, state)
                };

                // Create a new connection for the configured HTTP version. For
                // now we only support HTTP/1.1. This will be expanded to
                // support HTTP/2 in the future.
                let connection = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service)
                    .with_upgrades();

                // Clone the watch channel so that we can notify the connection
                // to initiate a graceful shutdown process before the server
                // exits.
                let mut shutdown_rx = shutdown_rx.clone();

                // Spawn a task to serve the connection.
                connections.spawn(async move {
                    // Define connection as mutable.
                    let mut connection = connection;

                    // Pin the connection on the stack so it can be polled.
                    //
                    // This is required by the tokio::select! macro as well as
                    // initiating a graceful shutdown process for the connection.
                    let mut pinned_connection = Pin::new(&mut connection);

                    // Poll the connection until it is closed or a graceful
                    // shutdown process is initiated.
                    let result = tokio::select! {
                        // Wait for the connection to close. This is the typical
                        // path that the code should take while the server is
                        // running.
                        result = &mut pinned_connection => result,

                        // Otherwise, wait until `shutdown_rx` is notified that
                        // the server will shutdown and initiate a graceful
                        // shutdown process for the connection.
                        _ = shutdown_rx.changed() => {
                            // Initiate the graceful shutdown process for the
                            // connection.
                            pinned_connection.as_mut().graceful_shutdown();

                            // Wait for the connection to close.
                            pinned_connection.await
                        }
                    };

                    // Release the permit back to the semaphore.
                    drop(permit);

                    // Return the result of the connection future.
                    Ok(result?)
                });
            }

            // Otherwise, wait for a "Ctrl-C" notification to be sent to the
            // process.
            _ = ctrl_c.as_mut() => {
                // Notify inflight connections to initiate a graceful shutdown.
                shutdown_tx.send(true)?;

                // Break out of the loop to stop accepting new connections.
                break;
            }
        }

        // Remove any handles that may have finished.
        while let Some(result) = connections.try_join_next() {
            if let Err(error) = result {
                //
                // TODO:
                //
                // Include tracing information about the connection
                // error that occurred. For now, we'll just print
                // the error to stderr if we're in debug mode.
                //
                if cfg!(debug_assertions) {
                    eprintln!("Error: {}", error);
                }
            }
        }
    }

    tokio::select! {
        // Wait for all inflight connection to finish. If all connections close
        // before the graceful shutdown timeout, return without an error. For
        // unix-based systems, this translates to a 0 exit code.
        _ = shutdown(connections) => Ok(()),

        // Otherwise, return an error if we're unable to close all connections
        // before the graceful shutdown timeout, return an error. For unix-based
        // systems, this translates to a 1 exit code.
        _ = time::sleep(shutdown_timeout) => {
            Err(Error::new("server exited before all connections were closed.".to_string()))
        }
    }
}

async fn accept(
    listener: &TcpListener,
    semaphore: &Arc<Semaphore>,
) -> Result<(OwnedSemaphorePermit, (TcpStream, SocketAddr)), Error> {
    // Acquire a permit from the semaphore.
    let permit = semaphore.clone().acquire_many_owned(2).await?;

    // Attempt to accept a new connection from the TCP listener.
    match listener.accept().await {
        Ok(accepted) => Ok((permit, accepted)),
        Err(error) => {
            // Release the permit back to the semaphore.
            drop(permit);

            //
            // TODO:
            //
            // Include tracing information about why the connection could not
            // be accepted.
            //
            Err(error.into())
        }
    }
}

async fn shutdown(connections: JoinSet<Result<(), Error>>) -> Result<(), Error> {
    if cfg!(debug_assertions) {
        eprintln!(
            "waiting for {} inflight connection(s) to close...",
            connections.len()
        );
    }

    // Wait for all inflight connections to close.
    for result in connections.join_all().await {
        // Propagate individual connection errors that may have occurred.
        result?;
    }

    Ok(())
}
