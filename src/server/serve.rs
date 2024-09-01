use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::net::SocketAddr;
use std::pin::pin;
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
    response_timeout: Duration,
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
{
    // Create a vector to store the join handles of the spawned tasks. We'll
    // periodically check if any of the tasks have finished and remove them.
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mut signal = pin!(tokio::signal::ctrl_c());

    let (tx, rx) = watch::channel(false);

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


                let mut rx = rx.clone();

                // Create a hyper service to serve the incoming connection. We'll move
                // the service into a tokio task to distribute the load across multiple
                // threads.
                let service = {
                    let router = Arc::clone(&router);
                    let state = Arc::clone(&state);

                    Service::new(router, state, response_timeout)
                };

                // Create a new connection for the configured HTTP version. For
                // now we only support HTTP/1.1. This will be expanded to
                // support HTTP/2 in the future.
                let connection = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(TokioIo::new(stream), service);

                // Spawn a tokio task to serve the connection in a separate thread.
                handles.push(task::spawn(async move {
                    let mut connection = pin!(connection);

                    tokio::select! {
                        result = connection.as_mut() => {
                            drop(permit);

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
                        _ = rx.changed() => {
                            drop(permit);
                            connection.graceful_shutdown();
                        }
                    }
                }));
            }
            _ = signal.as_mut() => {
                tx.send(true)?;
                break;
            }
        }
    }

    let shutdown = async {
        while !handles.is_empty() {
            handles.retain(|handle| !handle.is_finished());
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Waiting for {} connections to close...", handles.len());
        }
    };

    tokio::select! {
        _ = shutdown => Ok(()),
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
            Err(Error::new("process exited before all connections were closed.".to_string()))
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
