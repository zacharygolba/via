use hyper::server::conn::http1;
use hyper_util::rt::TokioTimer;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::{self, JoinHandle};
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
    // Create a vector to store the join handles of the spawned tasks. We'll
    // periodically check if any of the tasks have finished and remove them.
    let mut inflight: Vec<JoinHandle<()>> = Vec::new();

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
        // Remove any handles that have finished.
        inflight.retain(|handle| !handle.is_finished());

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
                let stream = Box::pin(stream);

                // Clone the watch channel so that we can notify the connection
                // to initiate a graceful shutdown process before the server
                // exits.
                let mut shutdown_rx = shutdown_rx.clone();

                // Create a new connection for the configured HTTP version. For
                // now we only support HTTP/1.1. This will be expanded to
                // support HTTP/2 in the future.
                let connection = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(
                        // Wrap the TcpStream in a type that implements hyper's
                        // IO traits.
                        IoStream::new(stream),
                        // Create a hyper service to serve the incoming connection.
                        // We'll move the service into a tokio task to distribute
                        // the load across multiple threads.
                        Service::new(Arc::clone(&router), Arc::clone(&state))
                    );

                // Spawn a tokio task to serve the connection in a separate thread.
                inflight.push(task::spawn(async move {
                    let mut connection = connection;
                    let mut conn_mut = Pin::new(&mut connection);

                    tokio::select! {
                        // Wait for the connection to close. This is the typical
                        // path that the code should take while the server is
                        // running.
                        result = conn_mut.as_mut() => {
                            // The connection has been closed. Drop the semaphore
                            // permit to allow another connection to be accepted.
                            drop(permit);

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

                        // Otherwise, wait until `shutdown_rx` is notified that
                        // the server will shutdown and initiate a graceful
                        // shutdown process for the inflight connection.
                        _ = shutdown_rx.changed() => {
                            // Initiate the graceful shutdown process for the
                            // connection.
                            conn_mut.graceful_shutdown();
                        }
                    }
                }));
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
    }

    tokio::select! {
        // Wait for all inflight connection to finish. If all connections close
        // before the graceful shutdown timeout, return without an error. For
        // unix-based systems, this translates to a 0 exit code.
        _ = shutdown(&mut inflight) => Ok(()),

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

async fn shutdown(inflight: &mut Vec<JoinHandle<()>>) {
    if cfg!(debug_assertions) {
        eprintln!(
            "waiting for {} inflight connection(s) to close...",
            inflight.len()
        );
    }

    while let Some(handle) = inflight.pop() {
        let _ = handle.await;
    }
}
