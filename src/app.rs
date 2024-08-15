use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::body::Pollable;
use crate::event::{Event, EventListener};
use crate::middleware::BoxFuture;
use crate::router::{Endpoint, Router};
use crate::{Error, Middleware, Request, Response, Result};

pub struct App<State> {
    router: Router<State>,
    state: Arc<State>,
}

struct FutureResponse {
    future: BoxFuture<Result<Response, Error>>,
}

pub fn app<State>(state: State) -> App<State>
where
    State: Send + Sync + 'static,
{
    App {
        router: Router::new(),
        state: Arc::new(state),
    }
}

async fn serve<State>(
    event_listener: Arc<EventListener>,
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

        // Clone `event_listener`, `router`, and `state` so they can be moved
        // into the async block passed to `tokio::spawn`. This is required in
        // order for us to be able to serve multiple connections concurrently.
        // The performance implications of cloning these values should be
        // negligible in the context of a web server since they are all `Arc`
        // pointers and the underlying values are not actually being cloned.
        let event_listener = Arc::clone(&event_listener);
        let router = Arc::clone(&router);
        let state = Arc::clone(&state);

        // Spawn a tokio task to serve multiple connections concurrently.
        tokio::spawn(async move {
            let service = service_fn(|incoming| {
                // Wrap the incoming request in with `via::Request`.
                let request = Request::new(
                    incoming,
                    // Clone `state` so it can be moved into the wrapped request.
                    // In the near future, we may want to consider passing
                    // `&Arc<State>` to request instead of `Arc<State>`.
                    Arc::clone(&state),
                    // Clone `event_listener` so it can be moved into the wrapped
                    // request. In the near future, we may want to consider
                    // passing `&Arc<EventListener>` to request instead of
                    // `Arc<EventListener>`.
                    Arc::clone(&event_listener),
                );

                FutureResponse {
                    // Unwind the middleware stack for `request`. Store the
                    // future returned from the middleware stack so it can be
                    // polled. Once the future is ready, we'll unwrap the inner
                    // response so it can be returned to the client.
                    future: router.respond_to(request),
                }
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
                event_listener.call(event);
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

    pub async fn listen<T, F>(self, address: T, event_listener: F) -> Result<()>
    where
        T: ToSocketAddrs,
        F: Fn(Event) + Send + Sync + 'static,
    {
        let event_listener = Arc::new(EventListener::new(event_listener));
        let tcp_listener = TcpListener::bind(address).await?;

        let address = tcp_listener.local_addr()?;
        let router = Arc::new(self.router);
        let state = self.state;

        // Notify the event listener that the server is ready to accept
        // incoming connections at the address to which the TCP listener
        // is bound.
        event_listener.call(Event::ServerReady(&address));

        // Serve incoming connections from the TCP listener.
        serve(event_listener, tcp_listener, router, state).await
    }
}

impl Future for FutureResponse {
    type Output = Result<http::Response<Pollable>, Infallible>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        this.future.as_mut().poll(context).map(|result| {
            let response = result.unwrap_or_else(|_| {
                // TODO: Log the error.
                // TODO: Warn about missing error boundary in debug builds.
                Response::internal_server_error()
            });

            Ok(response.into_inner())
        })
    }
}
