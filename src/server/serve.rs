use hyper::server::conn;
use hyper::service::service_fn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use crate::body::RequestBody;
use crate::error::BoxError;
use crate::request::Request;
use crate::router::{Router, RouterError};
use crate::server::io_stream::IoStream;

pub async fn serve<T, A>(
    listener: TcpListener,
    mut acceptor: A,
    state: T,
    router: Router<T>, // Router holds no references to T.
    max_connections: usize,
    max_request_size: usize,
    shutdown_timeout: Duration,
) -> Result<ExitCode, BoxError>
where
    T: Send + Sync + 'static,
    A: Acceptor + Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    let state = Arc::new(state);

    let router = Arc::new(router);

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let (shutdown_tx, mut shutdown_rx) = watch::channel(None);

    // Spawn a task to wait for the `ctrl_c` future to resolve.
    tokio::spawn({
        let ctrl_c_future = signal::ctrl_c();
        let shutdown_tx = shutdown_tx.clone();

        async move {
            if ctrl_c_future.await.is_err() {
                eprintln!("unable to register the 'ctrl-c' signal.");
            } else if shutdown_tx.send(Some(false)).is_err() {
                eprintln!("unable to notify connections to shutdown.");
            }
        }
    });

    // Start accepting incoming connections.
    let exit_code = 'accept: loop {
        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_owned().await?;

        // Get a weak reference to the state passed to the via::app function so
        // it can be moved in to the connection task.
        let state = Arc::downgrade(&state);

        // Clone the app so it can be moved into the connection task to serve
        // the connection.
        let router = Arc::clone(&router);

        // Wait for something interesting to happen.
        let io = loop {
            tokio::select! {
                // A graceful shutdown was requested.
                _ = shutdown_rx.changed() => {
                    // Break out of the accept loop with the corrosponding exit code.
                    break 'accept match *shutdown_rx.borrow_and_update() {
                        Some(false) => ExitCode::from(0),
                        Some(true) | None => ExitCode::from(1),
                    }
                }

                // A new connection is ready to be accepted.
                result = listener.accept() => match result {
                    // Accept the stream from the acceptor.
                    Ok((stream, _addr)) => break match acceptor.accept(stream).await {
                        Ok(accepted) => IoStream::new(accepted),
                        Err(error) => {
                            let _ = &error; // Placeholder for tracing...
                            continue;
                        }
                    },
                    Err(error) => {
                        let _ = &error; // Placeholder for tracing...
                    }
                },

                // We have idle time. Join any inflight connections that may
                // have finished.
                _ = connections.join_next(), if !connections.is_empty() => {
                    while connections.try_join_next().is_some() {}
                }
            }
        };

        // Clone the watch sender so connections can notify the main thread
        // if an unrecoverable error is encountered.
        let shutdown_tx = shutdown_tx.clone();

        // Clone the watch channel so that we can notify the connection
        // task when initiate a graceful shutdown process before the server
        // exits.
        let mut shutdown_rx = shutdown_rx.clone();

        // Spawn a task to serve the connection.
        connections.spawn(async move {
            // Define a hyper service to serve the incoming request.
            let service = service_fn(|request| {
                // Route request to the corresponding middleware stack.
                let future = router.visit(request.uri().path()).map(|(params, next)| {
                    // Call the corresponding middleware stack to get a future
                    // that resolves to a response.
                    next.call(Request::new(
                        // Share a weak reference of state with Request.
                        Weak::clone(&state),
                        // Ownership of path params is given to Request.
                        params,
                        // Limit the length of the request body to max_request_size.
                        request.map(|body| RequestBody::new(max_request_size, body).into()),
                    ))
                });

                async {
                    // If the request was routed successfully, await the response
                    // future. If the future resolved with an error, generate a
                    // response from it.
                    //
                    // If the request was not routed successfully, immediately
                    // return so the connection can be closed and the server
                    // exit.
                    let response = future?.await.unwrap_or_else(|error| error.into());

                    // Send the generated http::Response over the socket.
                    Ok::<_, RouterError>(response.into_http_response())
                }
            });

            // Create a new HTTP/2 connection.
            #[cfg(feature = "http2")]
            let mut connection = conn::http2::Builder::new(TokioExecutor::new())
                .timer(TokioTimer::new())
                .serve_connection(io, service);

            // Create a new HTTP/1.1 connection.
            #[cfg(all(feature = "http1", not(feature = "http2")))]
            let mut connection = conn::http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service)
                .with_upgrades();

            let mut pin_conn = Pin::new(&mut connection);

            // Serve the connection.
            if let Err(error) = tokio::select!(
                // Wait for the connection to close.
                result = &mut pin_conn => result,

                // Wait for the server to start a graceful shutdown. Then
                // initiate the same for the individual connection.
                _ = shutdown_rx.changed() => {
                    // The graceful_shutdown fn requires Pin<&mut Self>.
                    Pin::as_mut(&mut pin_conn).graceful_shutdown();

                    // Wait for the connection to close.
                    (&mut pin_conn).await
                }
            ) {
                let _ = &error; // Placeholder for tracing...
                if error.source().is_some_and(|e| e.is::<RouterError>()) {
                    let _ = shutdown_tx.send(Some(true));
                }
            }

            // Assert that the connection did not move.
            if cfg!(debug_assertions) {
                #[allow(clippy::let_underscore_future)]
                let _ = &mut connection;
            }

            // Return the permit back to the semaphore.
            drop(permit);
        });
    };

    tokio::select! {
        // Wait for inflight connection to close within the configured timeout.
        _ = shutdown(&mut connections) => Ok(exit_code),

        // Otherwise, return an error.
        _ = time::sleep(shutdown_timeout) => {
            let message = "server exited before all connections were closed".to_owned();
            Err(BoxError::from(message))
        }
    }
}

async fn shutdown(connections: &mut JoinSet<()>) {
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            let _ = &error; // Placeholder for tracing...
        }
    }
}
