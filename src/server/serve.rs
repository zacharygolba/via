use hyper::server::conn;
use hyper::service::service_fn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time;
use via_router::VisitError;

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::io_stream::IoStream;
use super::shutdown::wait_for_shutdown;
use crate::body::{HttpBody, RequestBody};
use crate::error::BoxError;
use crate::request::Request;
use crate::router::Router;

fn is_visit_error(error: &(dyn std::error::Error + 'static)) -> bool {
    error.is::<VisitError>()
}

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
    let (shutdown_task, shutdown_tx, mut shutdown_rx) = wait_for_shutdown();

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    // Start accepting incoming connections.
    let exit_code = 'accept: loop {
        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_owned().await?;

        // Wait for something interesting to happen.
        let (stream, _) = loop {
            tokio::select! {
                // Try to join inflight connections while we wait.
                _ = connections.join_next(), if !connections.is_empty() => {
                    // Continue to the next iteration.
                }

                // Wait for a new connection to be accepted.
                result = listener.accept() => match result {
                    Ok(accepted) => break accepted,
                    Err(error) => {
                        let _ = error; // Placeholder for tracing...
                        // Continue to the next iteration.
                    }
                },

                // Wait for a shutdown signal.
                _ = shutdown_rx.changed() => {
                    // Return the permit back to the semaphore.
                    drop(permit);

                    // Break out of the accept loop with the corrosponding exit code.
                    break 'accept match *shutdown_rx.borrow_and_update() {
                        // A scheduled shutdown was requested. An `ExitCode::SUCCESS` can
                        // be viewed as a confirmation that every request was served
                        // before exiting the event loop. Restart logic configured in a
                        // process manager such as upstart or systemd should be
                        // circumvented if the main process exits with
                        // `ExitCode::SUCCESS`.
                        Some(false) => ExitCode::from(0),

                        // An unrecoverable error occurred. An `ExitCode(1)` can be
                        // used to initiate restart logic configured in a process
                        // supervisor such as upstart or systemd.
                        Some(true) | None => ExitCode::from(1),
                    }
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
                    eprintln!("Error: {}", &error); // Placeholder for tracing...
                    drop(permit);
                    return;
                }
            };

            // Define a hyper service to serve the incoming request.
            let service = service_fn(|request| {
                // Destructure the incoming request into it's component parts.
                let (head, body) = request.into_parts();

                // Optionally allocate the request head on the heap.
                #[cfg(feature = "box-request-head")]
                let head = Box::new(head);

                // Limit the length of the request body to the configured max.
                let body = HttpBody::Original(RequestBody::new(max_request_size, body));

                // Route the request to the corresponding middleware stack.
                let result = router.lookup(head.uri.path()).map(|(params, next)| {
                    // Call the middleware stack for the matched routes.
                    next.call(Request::new(Arc::clone(&state), head, body, params))
                });

                async {
                    let mut response = result?.await.unwrap_or_else(|e| e.into());

                    // If any cookies changed during the request, serialize them to
                    // Set-Cookie headers and include them in the response headers.
                    response.set_cookie_headers();

                    // Unwrap the inner response type and let hyper take it from here.
                    Ok::<_, VisitError>(response.into_inner())
                }
            });

            // Create a new HTTP/2 connection.
            #[cfg(feature = "http2")]
            let mut connection = Box::pin(
                conn::http2::Builder::new(TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .serve_connection(io, service),
            );

            // Create a new HTTP/1.1 connection.
            #[cfg(all(feature = "http1", not(feature = "http2")))]
            let mut connection = Box::pin(
                conn::http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service)
                    .with_upgrades(),
            );

            // Serve the connection.
            if let Err(error) = tokio::select!(
                // Wait for the connection to close.
                result = &mut connection => result,

                // Wait for the server to start a graceful shutdown. Then
                // initiate the same for the individual connection.
                _ = shutdown_rx.changed() => {
                    // Get a mutable reference to the boxed connection.
                    let conn_mut = &mut connection;

                    // The graceful_shutdown requires Pin<&mut Self>.
                    Pin::as_mut(conn_mut).graceful_shutdown();

                    // Wait for the connection to close.
                    conn_mut.await
                }
            ) {
                eprintln!("Error: {}", &error); // Placeholder for tracing...
                if error.source().map_or(false, is_visit_error) {
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

    let shutdown_started_at = Instant::now();

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
            let elapsed_as_seconds = shutdown_started_at.elapsed().as_secs();
            let timeout_as_seconds = shutdown_timeout.as_secs();
            let remaining_timeout = timeout_as_seconds
                .checked_sub(elapsed_as_seconds)
                .map_or(Duration::from_secs(10), Duration::from_secs);

            // Wait for the shutdown task to complete before exiting the server.
            time::timeout(remaining_timeout, shutdown_task).await??;

            // Assert that every permit was returned back to the semaphore.
            if cfg!(debug_assertions) {
                assert_eq!(semaphore.available_permits(), max_connections);
            }

            // The shutdown_task completed within the timeout.
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
