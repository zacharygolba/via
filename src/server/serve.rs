use http::StatusCode;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::{task, time};

use crate::body::Pollable;
use crate::router::Router;
use crate::{Error, Request};

/// The request type used by our `hyper` service. This is the type that we will
/// use to create a `via::Request` that will be passed to the middleware stack.
type HttpRequest = http::Request<Incoming>;

/// The response type used by our `hyper` service. This is the type that we will
/// unwrap from the `via::Response` returned from the middleware stack.
type HttpResponse = http::Response<Pollable>;

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
    let mut handles = Vec::new();
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    loop {
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

        // Pass the returned stream to the `TokioIo` wrapper to convert
        // the stream into a tokio-compatible I/O stream.
        let io = TokioIo::new(stream);

        // Create a hyper service to serve the incoming connection.
        let service = {
            let router = Arc::clone(&router);
            let state = Arc::clone(&state);

            service_fn(move |request| {
                let state = Arc::clone(&state);

                serve_request(&router, state, request, response_timeout)
            })
        };

        // Create a new connection for the configured HTTP version. For
        // now we only support HTTP/1.1. This will be expanded to
        // support HTTP/2 in the future.
        let connection = http1::Builder::new()
            .timer(TokioTimer::new())
            .serve_connection(io, service);

        // Spawn a tokio task to serve multiple connections concurrently and push
        // the returned `JoinHandle` into the handles vector.
        handles.push(task::spawn(async move {
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

        // Remove any handles that have finished.
        handles.retain(|handle| !handle.is_finished());
    }
}

/// Serves an incoming request by routing it to the corresponding middleware
/// stack and returns a response.
fn serve_request<State>(
    router: &Router<State>,
    state: Arc<State>,
    request: HttpRequest,
    response_timeout: Duration,
) -> impl Future<Output = Result<HttpResponse, Infallible>>
where
    State: Send + Sync + 'static,
{
    // Get a Vec of routes that match the uri path.
    let matched_routes = router.lookup(request.uri().path());

    // Build the middleware stack for the request.
    let (params, next) = router.resolve(&matched_routes);

    // Wrap the incoming request in with `via::Request`.
    let request = Request::new(request, params, state);

    // Call the middleware stack and return a Future that resolves to a
    // Result<Response, Error>. If the response takes longer than the
    // configured timeout, respond with a 504 Gateway Timeout.
    let future = time::timeout(response_timeout, next.call(request));

    async {
        Ok(match future.await {
            // The response was generated successfully.
            Ok(Ok(response)) => response.into_inner(),
            // An occurred while generating the response.
            Ok(Err(error)) => error.into_response().into_inner(),
            // The response timed out.
            Err(_) => {
                let error = Error::with_status(
                    "The server is taking too long to respond. Please try again later.".to_owned(),
                    StatusCode::GATEWAY_TIMEOUT,
                );

                error.into_response().into_inner()
            }
        })
    }
}
