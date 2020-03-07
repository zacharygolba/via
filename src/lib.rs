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

#[derive(Default)]
pub struct App {
    router: Router,
    state: Extensions,
}

#[macro_export]
macro_rules! blocking {
    { $($handler:expr),* $(,)* } => {};
}

#[macro_export]
macro_rules! middleware {
    { $($handler:expr),* $(,)* } => {};
}

impl App {
    #[inline]
    pub fn new() -> App {
        Default::default()
    }

    #[inline]
    pub fn at(&mut self, path: &'static str) -> Location {
        Location {
            state: &mut self.state,
            value: self.router.at(path),
        }
    }

    #[inline]
    pub fn inject(&mut self, value: impl Send + Sync + 'static) {
        self.state.insert(value);
    }

    #[inline]
    pub fn mount(&mut self, service: impl Service) {
        self.at("/").mount(service);
    }

    #[inline]
    pub async fn listen(self) -> Result<()> {
        server::serve(self).await
    }
}
