#[macro_export]
macro_rules! bail {
    ($($tokens:tt)+) => {
        Err($crate::error::Bail::new(format!($($tokens)+)))?
    };
}

mod router;

pub mod error;
pub mod middleware;
pub mod prelude;
pub mod request;
pub mod response;

pub use http;

#[doc(inline)]
pub use crate::{
    error::{Error, ResultExt},
    middleware::{
        allow_method::{connect, delete, get, head, options, patch, post, put, trace},
        Middleware, Next,
    },
    request::Context,
    response::IntoResponse,
    router::Endpoint,
};

use futures::future::{FutureExt, Map};
use hyper::{server::conn::http1, service::Service};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};
use tokio::net::TcpListener;

use crate::{
    middleware::BoxFuture,
    request::{HyperRequest, PathParams},
    response::{HyperResponse, Response},
    router::Router,
};

pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

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

impl Service<HyperRequest> for AppServer {
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
