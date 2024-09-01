use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
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

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    loop {
        // Remove any handles that have finished.
        handles.retain(|handle| !handle.is_finished());

        // Create a hyper service to serve the incoming connection. We'll move
        // the service into a tokio task to distribute the load across multiple
        // threads.
        let service = {
            let router = Arc::clone(&router);
            let state = Arc::clone(&state);

            Service::new(router, state, response_timeout)
        };

        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_many_owned(2).await?;

        // Attempt to accept a new connection from the TCP listener.
        let (stream, _addr) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(_) => {
                drop(permit);
                //
                // TODO:
                //
                // Include tracing information about why the connection could not
                // be accepted.
                //
                continue;
            }
        };

        // Create a new connection for the configured HTTP version. For
        // now we only support HTTP/1.1. This will be expanded to
        // support HTTP/2 in the future.
        let connection = http1::Builder::new()
            .timer(TokioTimer::new())
            .serve_connection(TokioIo::new(stream), service);

        // Spawn a tokio task to serve the connection in a separate thread.
        handles.push(task::spawn(async {
            if let Err(error) = connection.await {
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

            drop(permit);
        }));
    }
}
