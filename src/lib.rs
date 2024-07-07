mod error;
mod event;
mod router;

pub mod middleware;
pub mod request;
pub mod response;

pub use http;

pub use crate::{
    error::{Error, Result},
    event::Event,
    middleware::{ErrorBoundary, Middleware, Next},
    request::Request,
    response::Response,
    router::Endpoint,
};

use http::Method;
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};
use tokio::net::TcpListener;

use crate::{
    event::EventListener,
    middleware::{AllowMethod, BoxFuture},
    router::Router,
};

pub struct App<State> {
    router: Router<State>,
    state: Arc<State>,
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

pub fn connect<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::CONNECT, middleware)
}

pub fn delete<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::DELETE, middleware)
}

pub fn get<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::GET, middleware)
}

pub fn head<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::HEAD, middleware)
}

pub fn options<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::OPTIONS, middleware)
}

pub fn patch<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::PATCH, middleware)
}

pub fn post<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::POST, middleware)
}

pub fn put<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::PUT, middleware)
}

pub fn trace<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::TRACE, middleware)
}

fn get_addr(sources: impl ToSocketAddrs) -> Result<SocketAddr> {
    match sources.to_socket_addrs()?.next() {
        Some(value) => Ok(value),
        None => todo!(),
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
        let app = Arc::new(self);
        let address = get_addr(address)?;
        let tcp_listener = TcpListener::bind(address).await?;
        let event_listener = EventListener::new(event_listener);

        // Notify the event listener that the server is ready to accept incoming
        // connections at the given address.
        event_listener.call(Event::ServerReady(&address));

        loop {
            // Accept a new connection and pass the returned stream to the
            // `TokioIo` wrapper to convert the stream into a tokio-compatible
            // I/O stream.
            let (stream, _) = tcp_listener.accept().await?;
            let io = TokioIo::new(stream);

            // Clone the `EventListener` and `App` instances by incrementing
            // the reference counts of the `Arc`.
            let event_listener = event_listener.clone();
            let app = Arc::clone(&app);

            // Spawn a tokio task to serve multiple connections concurrently.
            tokio::spawn(async move {
                let service = service_fn(|request| {
                    // Wrap the hyper request with `Request` as early as possible.
                    let request = {
                        let app_state = Arc::clone(&app.state);
                        let event_listener = event_listener.clone();
                        Request::new(request, app_state, event_listener)
                    };

                    // Delegate the request to the application to route the
                    // request to the appropriate middleware stack.
                    app.call(request, &event_listener)
                });

                if let Err(error) = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service)
                    .await
                {
                    // A connection error occured while serving the connection.
                    // Propagate the error to the event listener so it can be
                    // handled at the application level.
                    event_listener.call(Event::ConnectionError(&error.into()));
                }
            });
        }
    }

    async fn call(
        &self,
        mut request: Request<State>,
        event_listener: &EventListener,
    ) -> Result<http::Response<response::Body>, Infallible> {
        let next = self.router.visit(&mut request);
        let response = next.call(request).await.unwrap_or_else(|error| {
            error.into_infallible_response(|error| {
                // If the error was not able to be converted into a response,
                // with the configured error format (i.e json), fallback to a
                // plain text response and propagate the reason why the error
                // could not be converted to the event listener so it can be
                // handled at the application level.
                event_listener.call(Event::UncaughtError(&error));
            })
        });

        Ok(response.into_inner())
    }
}
