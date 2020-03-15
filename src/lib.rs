mod handler;
mod routing;
mod runtime;
mod server;
mod state;
mod util;

pub mod error;
pub mod plugin;
pub mod prelude;

use std::net::ToSocketAddrs;

pub(crate) use self::{
    handler::{ArcMiddleware, Request},
    routing::Routes,
    state::State,
};

#[doc(inline)]
pub use self::{error::Result, handler::*, routing::*, state::Value};
pub use codegen::*;
pub use http;

#[doc(hidden)]
pub use verbs;

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;

pub struct Application {
    routes: Routes,
    state: State,
}

#[macro_export]
macro_rules! middleware {
    { $($middleware:expr),* $(,)* } => {};
}

#[macro_export]
macro_rules! services {
    { $($service:expr),* $(,)* } => {};
}

#[inline]
pub fn new() -> Application {
    Application {
        routes: Default::default(),
        state: Default::default(),
    }
}

impl Application {
    #[inline]
    pub async fn listen(self, address: impl ToSocketAddrs) -> Result<()> {
        if let Some(address) = address.to_socket_addrs()?.next() {
            server::serve(self, address).await
        } else {
            todo!()
        }
    }

    #[inline]
    pub fn middleware(&mut self, middleware: impl Middleware) {
        self.namespace("/").middleware(middleware);
    }

    #[inline]
    pub fn namespace(&mut self, pattern: &'static str) -> Router {
        self.routes.namespace(&mut self.state, pattern)
    }

    #[inline]
    pub fn service(&mut self, service: impl Service) {
        self.namespace("/").service(service);
    }

    #[inline]
    pub fn state(&mut self, value: impl Value) {
        self.state.insert(value);
    }
}

impl Application {
    #[inline]
    pub(crate) fn context(&self, request: Request) -> Context {
        (self.state.clone(), request).into()
    }
}
