mod error;
mod handler;
mod routing;
mod runtime;
mod server;

pub mod helpers;
pub mod prelude;

use http::Extensions;
use std::net::ToSocketAddrs;

pub use self::{error::*, handler::*, routing::*};
pub use codegen::*;
pub use http;
pub use verbs;

pub struct App {
    router: Router,
    state: Extensions,
}

/// A marker trait used to describe types that can be injected into the global
/// state of an application.
pub trait State: Send + Sync + 'static {}

#[macro_export]
macro_rules! middleware {
    { $($handler:expr),* $(,)* } => {};
}

#[inline]
pub fn new() -> App {
    App {
        router: Default::default(),
        state: Default::default(),
    }
}

impl App {
    #[inline]
    pub fn at(&mut self, path: &'static str) -> Location {
        self.router.at(&mut self.state, path)
    }

    #[inline]
    pub async fn listen(self, address: impl ToSocketAddrs) -> Result<()> {
        if let Some(address) = address.to_socket_addrs()?.next() {
            server::serve(self, address).await
        } else {
            todo!()
        }
    }

    #[inline]
    pub fn middleware(&mut self, handler: impl Middleware) {
        self.at("/").middleware(handler);
    }

    #[inline]
    pub fn service(&mut self, service: impl Service) {
        self.at("/").service(service);
    }

    #[inline]
    pub fn state(&mut self, value: impl State) {
        self.state.insert(value);
    }
}

impl<T: Send + Sync + 'static> State for T {}
