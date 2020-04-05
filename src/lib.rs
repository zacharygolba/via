#[macro_export]
macro_rules! bail {
    ($($tokens:tt)+) => {
        Err($crate::error::Bail {
            message: format!($($tokens)+)
        })?
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
pub use codegen::{action, service};

pub use http;
pub use verbs;

use self::{response::Response, routing::*, service::MakeService};
use futures::future::{FutureExt, Map};
use hyper::Server;
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
};

type CallFuture = Map<BoxFuture<Result>, fn(Result) -> Result<HttpResponse, Infallible>>;
type HttpRequest = http::Request<hyper::Body>;
type HttpResponse = http::Response<hyper::Body>;

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
pub type Result<T = response::Response, E = Error> = std::result::Result<T, E>;

#[macro_export]
macro_rules! includes {
    { $($middleware:expr),* $(,)* } => {};
}

#[macro_export]
macro_rules! mount {
    { $($service:expr),* $(,)* } => {};
}

#[macro_export]
macro_rules! only([$($method:ident),*] => {
    $crate::middleware::filter::only($($crate::verbs::Verb::$method)|*)
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

    pub async fn listen(self, address: impl ToSocketAddrs) -> Result<()> {
        let address = get_addr(address)?;
        let server = Server::bind(&address).serve(MakeService::from(self));
        let ctrlc = async {
            let message = "failed to install CTRL+C signal handler";
            tokio::signal::ctrl_c().await.expect(message);
        };

        println!("Server listening at http://{}", address);
        Ok(server.with_graceful_shutdown(ctrlc).await?)
    }

    fn call(&self, request: HttpRequest) -> CallFuture {
        let mut context = Context::from(request);
        let next = self.router.visit(&mut context);

        next.call(context)
            .map(|result| Ok(result.unwrap_or_else(Response::from).into()))
    }
}

impl Target for Application {
    fn mount<T: Service>(&mut self, service: T) {
        self.router.at("/").mount(service);
    }
}
