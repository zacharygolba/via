#[macro_export]
macro_rules! bail {
    ($($tokens:tt)+) => {
        Err($crate::error::Bail::new(format!($($tokens)+)))?
    };
}

pub mod error;
pub mod middleware;
pub mod prelude;
pub mod response;
pub mod routing;

#[doc(inline)]
pub use self::{
    error::{Error, ResultExt},
    middleware::{Context, Middleware, Next},
    response::IntoResponse,
};
use futures::{future::Map, FutureExt};
pub use http;

use http::Method;
use hyper::{server::conn::http1, service::Service};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};
use tokio::net::TcpListener;

use middleware::{context::PathParams, filter::MethodFilter};
use response::Response;
use routing::*;

type HttpRequest = http::Request<hyper::body::Incoming>;
type HttpResponse = http::Response<response::Body>;

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
pub type Result<T = response::Response, E = Error> = std::result::Result<T, E>;

pub struct App {
    router: Router,
}

struct AppServer {
    app: Arc<App>,
}

pub fn app() -> App {
    App {
        router: Router::new(),
    }
}

pub fn connect<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::CONNECT, middleware)
}

pub fn delete<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::DELETE, middleware)
}

pub fn get<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::GET, middleware)
}

pub fn head<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::HEAD, middleware)
}

pub fn options<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::OPTIONS, middleware)
}

pub fn patch<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::PATCH, middleware)
}

pub fn post<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::POST, middleware)
}

pub fn put<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::PUT, middleware)
}

pub fn trace<T: Middleware>(middleware: T) -> MethodFilter<T> {
    MethodFilter::new(Method::TRACE, middleware)
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

    pub fn include(&mut self, middleware: impl Middleware + 'static) -> &mut Self {
        self.at("/").include(middleware);
        self
    }

    pub async fn listen(self, address: impl ToSocketAddrs) -> Result<()> {
        let address = get_addr(address)?;
        let listener = TcpListener::bind(address).await?;
        let app_server = AppServer {
            app: Arc::new(self),
        };

        println!("Server listening at http://{}", address);

        loop {
            let (stream, _) = listener.accept().await?;
            let app_server = AppServer {
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

impl Service<HttpRequest> for AppServer {
    type Error = Infallible;
    type Future = Map<BoxFuture<Result>, fn(Result) -> Result<HttpResponse, Infallible>>;
    type Response = HttpResponse;

    fn call(&self, request: HttpRequest) -> Self::Future {
        let mut params = PathParams::new();
        let next = self.app.router.visit(&request, &mut params);

        next.call(Context::new(request, params))
            .map(|result| match result {
                Ok(response) => Ok(response.into()),
                Err(error) => Ok(Response::from(error).into()),
            })
    }
}
