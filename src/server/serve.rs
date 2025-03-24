use hyper::body::Incoming;
use hyper::server::conn;
use hyper::service::service_fn;
use hyper_util::rt::TokioTimer;
use std::error::Error;
use std::future::Future;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::server::ServerContext;
use crate::body::{HttpBody, ResponseBody};
use crate::error::DynError;
use crate::request::Request;
use crate::router::{MatchWhen, Router};
use crate::server::io_stream::IoStream;
use crate::{Next, Response};

pub async fn serve<A, T>(
    listener: TcpListener,
    acceptor: A,
    context: ServerContext<T>,
) -> Result<ExitCode, DynError>
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(context.max_connections));

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    let (shutdown_tx, mut shutdown_rx) = watch::channel(None);

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

    // Wrap ServerContext with arc so it can be cloned into the connection task.
    let context = Arc::new(context);

    // Start accepting incoming connections.
    let exit_code = loop {
        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_owned().await?;

        // Clone the acceptor so it can be moved into the connectiont task.
        let mut acceptor = acceptor.clone();

        // Clone ServerContext so it can be moved into the connection task.
        let context = Arc::clone(&context);

        // Wait for something interesting to happen.
        let (stream, _address) = tokio::select! {
            // A graceful shutdown was requested.
            _ = shutdown_rx.changed() => {
                // Break out of the accept loop with the corrosponding exit code.
                break match *shutdown_rx.borrow_and_update() {
                    Some(false) => ExitCode::from(0),
                    Some(true) | None => ExitCode::from(1),
                }
            }

            // A new connection is ready to be accepted.
            result = listener.accept() => match result {
                // Accept the stream from the acceptor.
                Ok(accepted) => accepted,
                Err(_) => {
                    // Placeholder for tracing...
                    continue;
                }
            },

            // We have idle time. Join any inflight connections that may
            // have finished.
            _ = connections.join_next(), if !connections.is_empty() => {
                while connections.try_join_next().is_some() {}
                continue;
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
            let io = match acceptor.accept(stream).await {
                Ok(accepted) => IoStream::new(accepted),
                Err(_) => {
                    // Placeholder for tracing...
                    drop(permit);
                    return;
                }
            };

            // Create a service from the provided closure to serve the request.
            let service = service_fn(|request| {
                run(
                    Arc::clone(&context.state),
                    &context.router,
                    context.max_body_size,
                    request,
                )
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

            // Serve the connection.
            if let Err(error) = tokio::select!(
                // Wait for the connection to close.
                result = &mut connection => result,

                // Wait for the server to start a graceful shutdown. Then
                // initiate the same for the individual connection.
                _ = shutdown_rx.changed() => {
                    let mut connection = Pin::new(&mut connection);

                    // The graceful_shutdown fn requires Pin<&mut Self>.
                    connection.as_mut().graceful_shutdown();

                    // Wait for the connection to close.
                    connection.await
                }
            ) {
                if error.source().is_some_and(|e| e.is::<via_router::Error>()) {
                    let _ = shutdown_tx.send(Some(true));
                } else {
                    // Placeholder for tracing...
                }
            }

            // Return the permit back to the semaphore.
            drop(permit);
        });
    };

    tokio::select! {
        // Wait for inflight connection to close within the configured timeout.
        _ = shutdown(&mut connections) => Ok(exit_code),

        // Otherwise, return an error.
        _ = time::sleep(context.shutdown_timeout) => {
            Err("server exited before all connections were closed".into())
        }
    }
}

fn run<T>(
    state: Arc<T>,
    router: &Router<T>,
    max_body_size: usize,
    incoming_request: http::Request<Incoming>,
) -> impl Future<Output = Result<http::Response<HttpBody<ResponseBody>>, via_router::Error>> {
    let mut request = {
        let (head, body) = incoming_request.into_parts();
        Request::new(max_body_size, state, head, body)
    };

    let result = 'router: {
        let mut next = Next::new();

        for (key, range) in router.visit(request.uri().path()) {
            let found = match router.resolve(key) {
                Ok(resolved) => resolved,
                Err(error) => break 'router Err(error),
            };

            if let Some(name) = found.param.cloned() {
                request.params_mut().push((name, range));
            }

            if let Some(route) = found.route {
                let middleware = route.iter().filter_map(|when| match when {
                    MatchWhen::Partial(partial) => Some(partial),
                    MatchWhen::Exact(exact) => {
                        if found.exact {
                            Some(exact)
                        } else {
                            None
                        }
                    }
                });

                next.stack_mut().extend(middleware.cloned());
            }
        }

        Ok(next.call(request))
    };

    async {
        // If an error occurs due to a failed integrity check in
        // the router, immediately return with an error so the
        // connection can be closed and the server exit.
        //
        // Otherwise, await the future to get a response from the
        // application.
        Ok::<_, via_router::Error>(match result?.await {
            // The request was routed successfully and the
            // application generated a response without error.
            Ok(response) => response.into_inner(),

            // The request was routed successfully but an error
            // occurred in the application.
            Err(error) => Response::from(error).into_inner(),
        })
    }
}

async fn shutdown(connections: &mut JoinSet<()>) {
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            let _ = &error; // Placeholder for tracing...
        }
    }
}
