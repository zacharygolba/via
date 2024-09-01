use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::{self, JoinHandle};

use super::service::Service;
use crate::router::Router;
use crate::Error;

pub async fn serve<State>(
    state: Arc<State>,
    router: Arc<Router<State>>,
    listener: TcpListener,
    max_connections: usize,
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
{
    // Create a vector to store the join handles of the spawned tasks. We'll
    // periodically check if any of the tasks have finished and remove them.
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    // Get a Future that resolves when the user sends a SIGINT signal to the
    // process.
    let mut ctrl_c = Box::pin(tokio::signal::ctrl_c());

    // Create a watch channel to notify the server to initiate a graceful
    // shutdown.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    loop {
        // Remove any handles that have finished.
        handles.retain(|handle| !handle.is_finished());

        tokio::select! {
            result = accept(&listener, &semaphore) => {
                let (permit, (stream, _addr)) = match result {
                    Ok(accepted) => accepted,
                    Err(_) => {
                        //
                        // TODO:
                        //
                        // Include tracing information about why the connection could not
                        // be accepted.
                        //
                        continue;
                    }
                };

                // Clone the watch channel so that we can notify the connection
                // to initiate a graceful shutdown process before the server
                // exits.
                let mut shutdown_rx = shutdown_rx.clone();

                // Create a hyper service to serve the incoming connection. We'll move
                // the service into a tokio task to distribute the load across multiple
                // threads.
                let service = Service::new(Arc::clone(&router), Arc::clone(&state));

                // Create a new connection for the configured HTTP version. For
                // now we only support HTTP/1.1. This will be expanded to
                // support HTTP/2 in the future.
                let connection = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(TokioIo::new(stream), service);

                // Spawn a tokio task to serve the connection in a separate thread.
                handles.push(task::spawn(async move {
                    let mut connection = connection;
                    let mut conn_mut = Pin::new(&mut connection);

                    tokio::select! {
                        result = conn_mut.as_mut() => {
                            if let Err(error) = result {
                                //
                                // TODO:
                                //
                                // Replace eprintln with pretty_env_logger or something similar.
                                // We should also determine if this is how we want to handle
                                // connection errors long-term.
                                //
                                if cfg!(debug_assertions) {
                                    eprintln!("Error: {}", error);
                                }
                            }
                        }
                        _ = shutdown_rx.changed() => {
                            conn_mut.graceful_shutdown();
                        }
                    }

                    drop(permit);
                }));
            }
            _ = ctrl_c.as_mut() => {
                shutdown_tx.send(true)?;
                break;
            }
        }
    }

    let shutdown = async {
        while !handles.is_empty() {
            handles.retain(|handle| !handle.is_finished());
            tokio::time::sleep(Duration::from_secs(1)).await;
            if cfg!(debug_assertions) {
                println!("waiting for {} connections to close...", handles.len());
            }
        }
    };

    tokio::select! {
        _ = shutdown => Ok(()),
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
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
