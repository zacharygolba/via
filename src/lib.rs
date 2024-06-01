#[macro_export]
macro_rules! bail {
    ($($tokens:tt)+) => {
        Err($crate::error::Bail::new(format!($($tokens)+)))?
    };
}

mod service;

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
    response::Respond,
};
pub use codegen::{endpoint, service};
pub use http;
pub use router::Verb;

use futures::future::{FutureExt, Map};
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
};
use tokio::net::TcpListener;

use self::{response::Response, routing::*};

type CallFuture = Map<BoxFuture<Result>, fn(Result) -> Result<HttpResponse, Infallible>>;
type HttpRequest = http::Request<hyper::body::Incoming>;
type HttpResponse = http::Response<response::Body>;

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
pub type Result<T = response::Response, E = Error> = std::result::Result<T, E>;

#[macro_export]
macro_rules! includes {
    { $($middleware:expr),* $(,)* } => {};
}

#[macro_export]
macro_rules! delegate {
    { $($service:expr),* $(,)* } => {};
}

#[macro_export]
macro_rules! only([$($method:ident),*] => {
    $crate::middleware::filter::only($($crate::Verb::$method)|*)
});

pub struct Application {
    router: Router,
}

pub fn new() -> Application {
    Application {
        router: Default::default(),
    }
}

fn get_addr(sources: impl ToSocketAddrs) -> Result<SocketAddr> {
    match sources.to_socket_addrs()?.next() {
        Some(value) => Ok(value),
        None => todo!(),
    }
}

impl Application {
    pub fn at(&mut self, pattern: &'static str) -> Location {
        self.router.at(pattern)
    }

    pub fn include(&mut self, middleware: impl Middleware) -> &mut Self {
        self.at("/").include(middleware);
        self
    }

    pub async fn listen(self, address: impl ToSocketAddrs) -> Result<()> {
        use crate::service::Service;

        let address = get_addr(address)?;
        let listener = TcpListener::bind(address).await?;
        let service = Service::from(self);
        // let ctrlc = async {
        //     let message = "failed to install CTRL+C signal handler";
        //     tokio::signal::ctrl_c().await.expect(message);
        // };

        println!("Server listening at http://{}", address);

        loop {
            let (stream, _) = listener.accept().await?;
            let instance = service.clone();

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

    fn call(&self, request: HttpRequest) -> CallFuture {
        let mut context = Context::from(request);
        let next = self.router.visit(&mut context);

        next.call(context)
            .map(|result| Ok(result.unwrap_or_else(Response::from).into()))
    }
}

impl Endpoint for Application {
    fn delegate<T: Service>(&mut self, service: T) {
        self.router.at("/").delegate(service);
    }
}
