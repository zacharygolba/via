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

use futures::future::{FutureExt, Map};
use http::Method;
use hyper::{server::conn::http1, service::Service};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};
use tokio::net::TcpListener;

use crate::{
    middleware::{AllowMethod, BoxFuture},
    request::{HyperRequest, PathParams},
    response::HyperResponse,
    router::Router,
};

pub struct Application {
    router: Router,
}

struct ApplicationServer {
    app: Arc<Application>,
}

pub fn app() -> Application {
    Application {
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

impl Application {
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
        let address = get_addr(address)?;
        let listener = TcpListener::bind(address).await?;
        let app_server = ApplicationServer {
            app: Arc::new(self),
        };

        println!("Server listening at http://{}", address);

        loop {
            let (stream, _) = listener.accept().await?;
            let app_server = ApplicationServer {
                app: Arc::clone(&app_server.app),
            };

            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.
            let io = TokioIo::new(stream);

            // Spawn a tokio task to serve multiple connections concurrently
            tokio::task::spawn(async {
                // Finally, we bind the incoming connection to our `hello` service
                if let Err(err) = http1::Builder::new()
                    .timer(TokioTimer::new())
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(io, app_server)
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

impl Service<HyperRequest> for ApplicationServer {
    type Error = Infallible;
    type Future = Map<BoxFuture<Result>, fn(Result) -> Result<HyperResponse, Infallible>>;
    type Response = HyperResponse;

    fn call(&self, request: HyperRequest) -> Self::Future {
        let mut params = PathParams::new();
        let next = self.app.router.visit(&request, &mut params);

        next.call(Context::new(request, params))
            .map(|result| match result {
                Ok(response) => Ok(response.into_hyper_response()),
                Err(error) => Ok(error.into_response().unwrap().into_hyper_response()),
            })
    }
}
