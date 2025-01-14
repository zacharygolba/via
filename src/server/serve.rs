use hyper::server::conn;
use hyper::service::service_fn;
use hyper_util::rt::TokioTimer;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time;

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::io_stream::IoStream;
use super::shutdown::wait_for_shutdown;
use crate::error::BoxError;
use crate::request::{Request, RequestBody};
use crate::response::Response;
use crate::router::Router;

pub async fn serve<State, A>(
    listener: TcpListener,
    acceptor: A,
    state: Arc<State>,
    router: Arc<Router<State>>,
    max_connections: usize,
    max_request_size: usize,
    shutdown_timeout: Duration,
) -> Result<ExitCode, BoxError>
where
    State: Send + Sync + 'static,
    A: Acceptor + Send + Sync + 'static,
{
    let (shutdown_tx, shutdown_rx, shutdown_task) = wait_for_shutdown();

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    let exit_code = loop {
        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_owned().await?;

        // Clone the Arc around the router so it can be moved into the connection
        // task.
        let router = Arc::clone(&router);

        // Clone the Arc around the shared application state so it can be moved
        // into the connection task.
        let state = Arc::clone(&state);

        // Clone the acceptor so it can be moved into the task responsible for
        // serving individual connections.
        let acceptor = acceptor.clone();

        // Clone the watch sender so connections can notify the main thread if an
        // unrecoverable error is encountered.
        let shutdown_tx = shutdown_tx.clone();

        // Clone the watch channel so that we can notify the connection task when
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
                    // Define the acceptor as mutable. We do this so we can be
                    // confident that accept is only called within the connection
                    // task.
                    let mut acceptor = acceptor;

                    // Accept the stream from the acceptor. This is where the
                    // TLS handshake would occur if the acceptor is a
                    // TlsAcceptor.
                    let io = match acceptor.accept(stream).await {
                        Ok(accepted) => IoStream::new(accepted),
                        Err(_) => {
                            // Placeholder for tracing...
                            return;
                        }
                    };

                    let service = service_fn(|r| {
                        // Wrap the incoming request in with `via::Request`.
                        let mut request = {
                            // Destructure the incoming request into its
                            // component parts.
                            let (parts, incoming) = r.into_parts();

                            // Allocate the metadata associated with the request
                            // on the heap. This keeps the size of the request
                            // type small enough to pass around by value.
                            let parts = Box::new(parts);

                            // Convert the incoming body type to a RequestBody.
                            let body = RequestBody::new(
                                max_request_size,
                                incoming,
                            );

                            // Clone the shared application state so request can
                            // own a reference to it. This is a cheaper operation
                            // than going from Weak to Arc for any application
                            // that interacts with a connection pool or an
                            // external service.
                            let state = Arc::clone(&state);

                            Request::new(parts, body, state)
                        };

                        let future = match router.lookup(
                            request.parts.uri.path(),
                            &mut request.params
                        ) {
                            // Call the middleware stack for the resolved routes.
                            Ok(next) => next.call(request),

                            // Alert the main thread that we encountered an
                            // unrecoverable error.
                            Err(error) => {
                                let _ = shutdown_tx.send(Some(true));
                                let _ = error; // Placeholder for tracing...
                                Box::pin(async {
                                    Response::internal_server_error()
                                })
                            }
                        };

                        async {
                            Ok::<_, hyper::Error>(match future.await {
                                // The response was generated successfully.
                                Ok(response) => response.into_inner(),
                                // An occurred while generating the response.
                                Err(error) => Response::from(error).into_inner(),
                            })
                        }
                    });

                    // Create a new HTTP/2 connection.
                    #[cfg(feature = "http2")]
                    let mut connection =
                        conn::http2::Builder::new(TokioExecutor::new())
                            .timer(TokioTimer::new())
                            .serve_connection(io, service);

                    // Create a new HTTP/1.1 connection.
                    #[cfg(all(feature = "http1", not(feature = "http2")))]
                    let mut connection = conn::http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(io, service)
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

                    // Return the permit back to the semaphore.
                    drop(permit);

                    if let Err(error) = result {
                        // Placeholder for tracing...
                        let _ = error;
                    }
                });
            }

            // Otherwise, wait for a "Ctrl-C" signal to be sent to the process.
            _ = shutdown_rx.changed() => match *shutdown_rx.borrow_and_update() {
                // NOOP
                None => {},

                // An unrecoverable error occurred. An `ExitCode::FAILURE` can be
                // used to initiate restart logic configured in a process
                // supervisor such as upstart or systemd.
                Some(true) => break ExitCode::FAILURE,

                // A scheduled shutdown was requested. An `ExitCode::SUCCESS` can
                // be viewed as a confirmation that every request was served
                // before exiting the event loop. Restart logic configured in a
                // process manager such as upstart or systemd should be
                // circumvented if the main process exits with
                // `ExitCode::SUCCESS`.
                Some(false) => break ExitCode::SUCCESS,
            }
        }

        // Remove any handles that may have finished.
        while let Some(result) = connections.try_join_next() {
            if let Err(error) = result {
                // Placeholder for tracing...
                let _ = error;
            }
        }
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
