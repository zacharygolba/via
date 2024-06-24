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
    response::{IntoResponse, Response},
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
    request::{IncomingRequest, PathParams},
    response::OutgoingResponse,
    router::Router,
};

pub struct App {
    router: Router,
}

pub fn app() -> App {
    App {
        router: Router::new(),
    }
}

pub fn connect<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::CONNECT, middleware)
}

pub fn delete<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::DELETE, middleware)
}

pub fn get<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::GET, middleware)
}

pub fn head<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::HEAD, middleware)
}

pub fn options<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::OPTIONS, middleware)
}

pub fn patch<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::PATCH, middleware)
}

pub fn post<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::POST, middleware)
}

pub fn put<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::PUT, middleware)
}

pub fn trace<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::TRACE, middleware)
}

fn get_addr(sources: impl ToSocketAddrs) -> Result<SocketAddr> {
    match sources.to_socket_addrs()?.next() {
        Some(value) => Ok(value),
        None => todo!(),
    }
}

impl App {
    pub fn at(&mut self, pattern: &'static str) -> Endpoint {
        self.router.at(pattern)
    }

    pub fn include<T>(&mut self, middleware: T) -> &mut Self
    where
        T: Middleware,
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
                let service = service_fn(|mut request| {
                    // Include `EventListener` in the request extensions to
                    // propagate errors that occur inside an error boundary.
                    //
                    // This is necessary because the error boundary middleware
                    // may fail to convert an error into a response.
                    //
                    // For example, if an error should be serialized as JSON,
                    // but an additional error occurs during the serialization
                    // process, the error boundary will fall back to a plain
                    // text response with the error message as the response body.
                    // This behavior is intended to prevent infinite loops from
                    // occurring inside an error boundary.
                    //
                    // However, we still want to know that an error occurred
                    // inside the error boundary. So, we propagate the error
                    // to the event listener so it can be handled at the
                    // application level.
                    request.extensions_mut().insert(event_listener.clone());

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
        request: IncomingRequest,
        event_listener: &EventListener,
    ) -> Result<OutgoingResponse, Infallible> {
        let mut path_params = PathParams::new();

        let next = self.router.visit(&mut path_params, &request);
        let request = Request::new(request, path_params);
        let response = next.call(request).await.unwrap_or_else(|error| {
            error.into_infallible_response(|error| {
                // If the error was not able to be converted into a response,
                // with the configured error format (i.e json), fall back to
                // a plain text response and propagate the reason why the error
                // could not be converted to the event listener so it can be
                // handled at the application level.
                event_listener.call(Event::UncaughtError(&error));
            })
        });

        Ok(response.into_inner())
    }
}
