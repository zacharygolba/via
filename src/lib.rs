mod error;
mod router;

pub mod middleware;
pub mod request;
pub mod response;

pub use http;

pub use crate::{
    error::{Error, Result},
    middleware::{Middleware, Next},
    request::Context,
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

    pub async fn listen<T>(self, address: T) -> Result<()>
    where
        T: ToSocketAddrs,
    {
        let app = Arc::new(self);
        let address = get_addr(address)?;
        let listener = TcpListener::bind(address).await?;

        println!("Server listening at http://{}", address);

        loop {
            let (stream, _) = listener.accept().await?;
            let app = Arc::clone(&app);
            let io = TokioIo::new(stream);

            // Spawn a tokio task to serve multiple connections concurrently
            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service_fn(|request| app.call(request)))
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }

    async fn call(&self, request: IncomingRequest) -> Result<OutgoingResponse, Infallible> {
        let mut path_params = PathParams::new();
        let next = self.router.visit(&mut path_params, &request);
        let context = Context::new(request, path_params);

        match next.call(context).await {
            Ok(response) => Ok(response.into_inner()),
            // TODO:
            // Add a function that can safely convert an error into a response.
            // For example, if an error has a format of Format::Json, serialization
            // may fail. In this case, we'll want to log the serialization error and
            // then return the original error as a plain text response.
            Err(error) => Ok(error.into_response().unwrap().into_inner()),
        }
    }
}
