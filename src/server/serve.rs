use hyper::server::conn;
use hyper_util::rt::{TokioIo, TokioTimer};
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
use super::service::Service;
use super::shutdown::wait_for_shutdown;
use crate::error::BoxError;
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
                    // TLS handshake would occur if the acceptor is a TlsAcceptor.
                    let stream = match acceptor.accept(stream).await {
                        Ok(accepted) => accepted,
                        Err(_) => {
                            // Placeholder for tracing...
                            return;
                        }
                    };

                    // Create a new HTTP/2 connection.
                    #[cfg(feature = "http2")]
                    let mut connection = conn::http2::Builder::new(TokioExecutor::new())
                        .timer(TokioTimer::new())
                        .serve_connection(
                            TokioIo::new(stream),
                            Service::new(max_request_size, shutdown_tx, router, state),
                        );

                    // Create a new HTTP/1.1 connection.
                    #[cfg(all(feature = "http1", not(feature = "http2")))]
                    let mut connection = conn::http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(
                            TokioIo::new(stream),
                            Service::new(max_request_size, shutdown_tx, router, state),
                        )
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
            Err("server exited before all connections were closed.".to_owned().into())
        }
    }
}
