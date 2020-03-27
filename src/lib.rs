mod service;

pub mod middleware;
pub mod system {
    pub use super::{action, includes, mount, service};
    pub use super::{
        middleware::{self, Context, Middleware, Next},
        response::{self, Respond, Response},
        routing::Target,
        Error, Result,
    };
}

#[doc(inline)]
pub use self::middleware::{Context, Middleware, Next};
pub use codegen::*;
pub use core::{response, routing, BoxFuture, Error, Respond, Result};
pub use http;

use self::{routing::*, service::MakeService};
use futures::future::{FutureExt, Map};
use hyper::Server;
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
};

type CallFuture = Map<BoxFuture<Result>, fn(Result) -> Result<HttpResponse, Infallible>>;
type HttpRequest = http::Request<hyper::Body>;
type HttpResponse = http::Response<hyper::Body>;

#[macro_export]
macro_rules! includes {
    { $($middleware:expr),* $(,)* } => {};
}

#[macro_export]
macro_rules! mount {
    { $($service:expr),* $(,)* } => {};
}

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

        next.call(context).map(|result| match result {
            Ok(response) => Ok(response.into()),
            Err(error) => Ok(error.into()),
        })
    }
}

impl Target for Application {
    fn mount<T: Service>(&mut self, service: T) {
        self.router.at("/").mount(service);
    }
}
