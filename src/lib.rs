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
pub mod view;

#[doc(inline)]
pub use self::{
    error::{Error, ResultExt},
    middleware::{Context, Middleware, Next},
    response::IntoResponse,
};
pub use http;

use http::Method;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};
use tokio::net::TcpListener;

use middleware::filter::{self, MethodFilter};
use response::Response;
use routing::*;

type HttpRequest = http::Request<hyper::body::Incoming>;
type HttpResponse = http::Response<response::Body>;

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
pub type Result<T = response::Response, E = Error> = std::result::Result<T, E>;

pub struct Application {
    router: Router,
}

pub fn new() -> Application {
    Application {
        router: Router::new(),
    }
}

impl Application {
    pub fn at(&mut self, pattern: &'static str) -> Endpoint {
        self.router.at(pattern)
    }

    pub fn include(&mut self, middleware: impl Middleware + 'static) -> &mut Self {
        self.at("/").include(middleware);
        self
    }

    pub async fn listen(self, address: impl ToSocketAddrs) -> Result<()> {
        let app = Arc::new(self);
        let address = get_addr(address)?;
        let listener = TcpListener::bind(address).await?;
        let service_fn = hyper::service::service_fn(move |request| {
            let app = Arc::clone(&app);
            async move { app.call(request).await }
        });
        // let ctrlc = async {
        //     let message = "failed to install CTRL+C signal handler";
        //     tokio::signal::ctrl_c().await.expect(message);
        // };

        println!("Server listening at http://{}", address);

        loop {
            let (stream, _) = listener.accept().await?;
            let instance = service_fn.clone();

            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.
            let io = TokioIo::new(stream);

            // Spawn a tokio task to serve multiple connections concurrently
            tokio::task::spawn(async move {
                // Finally, we bind the incoming connection to our `hello` service
                if let Err(err) = http1::Builder::new()
                    .timer(TokioTimer::new())
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(io, instance)
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }

    async fn call(&self, request: HttpRequest) -> Result<HttpResponse, Infallible> {
        let mut context = Context::from(request);
        let next = self.router.visit(&mut context);

        match next.call(context).await {
            Ok(response) => Ok(response.into()),
            Err(error) => Ok(Response::from(error).into()),
        }
    }
}

pub fn app() -> Application {
    Application {
        router: Router::new(),
    }
}

pub fn connect<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::CONNECT)(middleware)
}

pub fn delete<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::DELETE)(middleware)
}

pub fn get<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::GET)(middleware)
}

pub fn head<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::HEAD)(middleware)
}

pub fn options<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::OPTIONS)(middleware)
}

pub fn patch<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::PATCH)(middleware)
}

pub fn post<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::POST)(middleware)
}

pub fn put<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::PUT)(middleware)
}

pub fn trace<T: Middleware>(middleware: T) -> MethodFilter<T> {
    filter::method(Method::TRACE)(middleware)
}

fn get_addr(sources: impl ToSocketAddrs) -> Result<SocketAddr> {
    match sources.to_socket_addrs()?.next() {
        Some(value) => Ok(value),
        None => todo!(),
    }
}
