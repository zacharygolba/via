use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::task;

use crate::body::Pollable;
use crate::router::Router;
use crate::server::TcpListener;
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
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
{
    let mut listener = listener;

    loop {
        // Accept a new connection from the TCP listener.
        let (stream, _, permit) = listener.accept().await?;
        // Pass the returned stream to the `TokioIo` wrapper to convert
        // the stream into a tokio-compatible I/O stream.
        let io = TokioIo::new(stream);

        // Clone the `state` so it can be moved into the task.
        let state = Arc::clone(&state);
        // Clone the `router` so it can be moved into the task.
        let router = Arc::clone(&router);

        // Spawn a tokio task to serve multiple connections concurrently.
        task::spawn(async move {
            // Create a hyper service from the `serve_request` function.
            let service = service_fn(|request| serve_request(&state, &router, request));

            // Create a new connection for the configured HTTP version. For
            // now we only support HTTP/1.1. This will be expanded to
            // support HTTP/2 in the future.
            let connection = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service);

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

            drop(permit)
        });
    }
}

/// Serves an incoming request by routing it to the corresponding middleware
/// stack and returns a response.
async fn serve_request<State>(
    state: &Arc<State>,
    router: &Arc<Router<State>>,
    request: HttpRequest,
) -> Result<HttpResponse, Infallible>
where
    State: Send + Sync + 'static,
{
    // Wrap the incoming request in with `via::Request`.
    //
    // Note:
    //
    // In the future, we may want to pass a reference to `state` to avoid having
    // to incremet the ref count of the `Arc` unnecessarily.
    //
    let mut request = Request::new(request, Arc::clone(state));

    // Route `request` to the corresponding middleware stack.
    let next = router.respond_to(&mut request);

    // Call the middleware stack and return a response.
    match next.call(request).await {
        Ok(response) => Ok(response.into_inner()),
        Err(error) => Ok(error.into_response().into_inner()),
    }
}
