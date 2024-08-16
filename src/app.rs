use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::body::Pollable;
use crate::event::{Event, EventCallback, EventListener};
use crate::router::{Endpoint, Router};
use crate::{Error, Middleware, Next, Request, Result};

pub struct App<State> {
    router: Router<State>,
    state: Arc<State>,
}

/// Constructs a new `App` with the provided `state`.
pub fn app<State>(state: State) -> App<State>
where
    State: Send + Sync + 'static,
{
    App {
        router: Router::new(),
        state: Arc::new(state),
    }
}

/// Unwinds the middleware stack at `next` for the provided `request`. Returns
/// a result containing an unwrapped `http::Response` that is compatible with
/// the a `hyper` service.
///
/// If an error occurs while unwinding the middleware stack, we'll attempt to
/// convert the error into a response. If the conversion fails, we'll fallback
/// to a plain text response that contains the error message.
///
/// If you are concerned about sensitive data leaking into the response body,
/// consider using [`ErrorBoundary::map`](crate::middleware::ErrorBoundary::map)
/// to redact sensitive data from the error message.
async fn run<State>(
    request: Request<State>,
    next: Next<State>,
) -> Result<http::Response<Pollable>, Infallible>
where
    State: Send + Sync + 'static,
{
    let response = next.call(request).await.unwrap_or_else(|error| {
        // Convert any potential errors that occured while unwinding the
        // middleware stack into a `Response`.
        error.try_into_response().unwrap_or_else(|(fallback, _)| {
            // TODO:
            // Warn about missing error boundary in debug builds. If
            // the middleware stack of an application is missing an
            // error boundary. We assume that the default fallback
            // response is sufficient for the application's needs.

            // Return the fallback response.
            fallback
        })
    });

    Ok(response.into_inner())
}

async fn serve<State>(
    event_listener: EventListener<State>,
    tcp_listener: TcpListener,
    router: Arc<Router<State>>,
    state: Arc<State>,
) -> Result<()>
where
    State: Send + Sync + 'static,
{
    loop {
        // Accept a new connection from the TCP listener.
        let (stream, _) = tcp_listener.accept().await?;
        // Pass the returned stream to the `TokioIo` wrapper to convert
        // the stream into a tokio-compatible I/O stream.
        let io = TokioIo::new(stream);

        // Clone `router` by incrementing the reference count of the `Arc`. This
        // allows us to share ownership of `router` across multiple threads.
        let router = Arc::clone(&router);
        // Clone `state` by incrementing the reference count of the `Arc`. This
        // allows us to share ownership of `state` across multiple threads.
        let state = Arc::clone(&state);

        // Spawn a tokio task to serve multiple connections concurrently.
        tokio::spawn(async move {
            let service = service_fn(|incoming| {
                let mut request = {
                    // Clone `state` so ownership can be shared with the request.
                    // In the future, we may want to pass a reference to `state`
                    // to avoid the incremeting the ref count of the `Arc`
                    // unnecessarily.
                    let state = Arc::clone(&state);

                    // Wrap the incoming request in with `via::Request`.
                    Request::new(incoming, state, event_listener)
                };
                // Route `request` to the corresponding middleware stack.
                let next = router.visit(&mut request);

                // Unwind the middleware stack and return a response.
                run(request, next)
            });

            // Create a new connection for the configured HTTP version. For
            // now we only support HTTP/1.1. This will be expanded to
            // support HTTP/2 in the future.
            let connection = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service);

            if let Err(error) = connection.await {
                let error = Error::from(error);
                let event = Event::ConnectionError(&error);

                // A connection error occured while serving the connection.
                // Propagate the error to the event listener so it can be
                // handled at the application level.
                event_listener.call(event, &state);
            }
        });
    }
}

impl<State> App<State>
where
    State: Send + Sync + 'static,
{
    pub fn at(&mut self, pattern: &'static str) -> Endpoint<State> {
        self.router.at(pattern)
    }

    pub fn include<T>(&mut self, middleware: T) -> &mut Self
    where
        T: Middleware<State>,
    {
        self.at("/").include(middleware);
        self
    }

    pub async fn listen<T>(self, address: T, event_listener: EventCallback<State>) -> Result<()>
    where
        T: ToSocketAddrs,
    {
        let event_listener = EventListener::new(event_listener);
        let tcp_listener = TcpListener::bind(address).await?;

        let address = tcp_listener.local_addr()?;
        let router = Arc::new(self.router);
        let state = self.state;

        // Notify the event listener that the server is ready to accept
        // incoming connections at the address to which the TCP listener
        // is bound.
        event_listener.call(Event::ServerReady(&address), &state);

        // Serve incoming connections from the TCP listener.
        serve(event_listener, tcp_listener, router, state).await
    }
}
