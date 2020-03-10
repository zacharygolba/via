mod error;
mod handler;
mod routing;
mod runtime;
mod server;

pub mod helpers;
pub mod prelude;

use http::Extensions;

pub use self::{error::*, handler::*, routing::*};
pub use codegen::*;
pub use http;
pub use verbs;

pub struct App {
    router: Router,
    state: Extensions,
}

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
    pub async fn listen(self) -> Result<()> {
        server::serve(self).await
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
    pub fn state<T>(&mut self, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.state.insert(value);
    }
}
