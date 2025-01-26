use hyper::server::conn;
use hyper::service::service_fn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;
use tokio::{signal, time};
use via_router::VisitError;

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::io_stream::IoStream;
use crate::body::{HttpBody, RequestBody};
use crate::error::BoxError;
use crate::middleware::Next;
use crate::request::{PathParams, Request};
use crate::router::Router;

pub async fn serve<T, A>(
    listener: TcpListener,
    acceptor: A,
    state: Arc<T>,
    router: Arc<Router<T>>,
    max_connections: usize,
    max_request_size: usize,
    shutdown_timeout: Duration,
) -> Result<ExitCode, BoxError>
where
    T: Send + Sync + 'static,
    A: Acceptor + Send + Sync + 'static,
{
    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let (shutdown_tx, mut shutdown_rx) = watch::channel(None);

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    // Spawn a task to wait for the `ctrl_c` future to resolve.
    tokio::spawn({
        let shutdown_tx = shutdown_tx.clone();

        async move {
            if signal::ctrl_c().await.is_err() {
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

        // Wait for something interesting to happen.
        let stream = loop {
            tokio::select! {
                // A graceful shutdown was requested.
                _ = shutdown_rx.changed() => {
                    // Return the permit back to the semaphore.
                    drop(permit);

                    // Break out of the accept loop with the corrosponding exit code.
                    break 'accept match *shutdown_rx.borrow_and_update() {
                        Some(false) => ExitCode::from(0),
                        Some(true) | None => ExitCode::from(1),
                    }
                }

                // A new connection is ready to be accepted.
                result = listener.accept() => match result {
                    Ok((stream, _address)) => break stream,
                    Err(error) => {
                        let _ = error; // Placeholder for tracing...
                        // Continue to the next iteration.
                    }
                },

                // We have idle time. Join any inflight connections that may
                // have finished.
                _ = connections.join_next(), if !connections.is_empty() => {
                    while connections.try_join_next().is_some() {}
                }
            }
        };

        // Clone the acceptor so it can be moved into the task responsible
        // for serving individual connections.
        let mut stream_acceptor = acceptor.clone();

        // Clone the watch channel so that we can notify the connection
        // task when initiate a graceful shutdown process before the server
        // exits.
        let mut shutdown_rx = shutdown_rx.clone();

        // Clone the watch sender so connections can notify the main thread
        // if an unrecoverable error is encountered.
        let shutdown_tx = shutdown_tx.clone();

        // Clone the Arc around the router so it can be moved into the
        // connection task.
        let router = Arc::clone(&router);

        // Clone the Arc around the shared application state so it can be
        // moved into the connection task.
        let state = Arc::clone(&state);

        // Spawn a task to serve the connection.
        connections.spawn(async move {
            // Accept the stream from the acceptor. This is where the TLS
            // handshake occurs if the acceptor is a TlsAcceptor.
            let io = match stream_acceptor.accept(stream).await {
                Ok(accepted) => IoStream::new(accepted),
                Err(error) => {
                    let _ = error; // Placeholder for tracing...
                    drop(permit);
                    return;
                }
            };

            // Define a hyper service to serve the incoming request.
            let service = service_fn(|r| {
                let mut request = {
                    // Destructure the incoming request into it's component parts.
                    let (head, body) = r.into_parts();

                    // Construct a via::Request from the component parts of r.
                    Request::new(
                        // Clone the arc pointer around the global application
                        // state that was passed to the via::app function.
                        Arc::clone(&state),
                        // Allocate for path params.
                        PathParams::new(Vec::with_capacity(3)),
                        // Take ownership of the request head.
                        head,
                        // Limit the length of the request body to max_request_size.
                        HttpBody::Original(RequestBody::new(max_request_size, body)),
                    )
                };

                let result = {
                    // Allocate a vec to store matched routes.
                    let mut next = Next::new(Vec::with_capacity(8));

                    // Get a mutable ref to path params and a str containing
                    // the request uri.
                    let (params, path) = request.params_mut_with_path();

                    // Route the request to the corresponding middleware stack.
                    match router.lookup(path, params, &mut next) {
                        // Call the middleware stack for the matched routes.
                        Ok(_) => Ok(next.call(request)),

                        // Close the connection and stop the server.
                        Err(e) => Err(e),
                    }
                };

                async {
                    // If the request was routed successfully, await the
                    // response future. If the future resolved with an error,
                    // generate a response from it.
                    //
                    // If the request was not routed successfully, immediately
                    // return so the connection can be closed and the server
                    // exit.
                    let mut response = result?.await.unwrap_or_else(|e| e.into());

                    // If any cookies changed during the request, serialize them to
                    // Set-Cookie headers and include them in the response headers.
                    response.set_cookie_headers();

                    // Unwrap the inner http::Response so it can be sent back
                    // to the client via IoStream.
                    Ok::<_, VisitError>(response.into_inner())
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
                if error.source().is_some_and(|e| e.is::<VisitError>()) {
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

    if cfg!(debug_assertions) {
        // TODO: Replace this with tracing.
        eprintln!(
            "waiting for {} inflight connection(s) to close...",
            connections.len()
        );
    }

    tokio::select! {
        // Wait for all inflight connection to finish. If all connections close
        // before the graceful shutdown timeout, return without an error. For
        // unix-based systems, this translates to a 0 exit code.
        _ = connections.join_all() => {
            Ok(exit_code)
        }

        // Otherwise, return an error if we're unable to close all connections
        // before the graceful shutdown timeout, return an error. For unix-based
        // systems, this translates to a 1 exit code.
        _ = time::sleep(shutdown_timeout) => {
            let message = "server exited before all connections were closed".to_owned();
            let error = BoxError::from(message);

            Err(error)
        }
    }
}
