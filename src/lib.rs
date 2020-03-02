mod error;
mod handler;
mod runtime;

pub mod helpers;
pub mod prelude;
pub mod routing;

use self::{routing::*, runtime::MakeService};
use http::Extensions;
use hyper::Server;

pub use self::{error::*, handler::*};
pub use codegen::*;
pub use http;

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
    pub fn mount(&mut self, mount: impl Mount) {
        self.at("/").mount(mount);
    }

    #[inline]
    pub fn listen(self) -> Result<(), Error> {
        use tokio::runtime::Runtime;

        let address = "0.0.0.0:8080".parse()?;

        Runtime::new()?.block_on(async {
            let service = MakeService::from(self);
            let server = Server::bind(&address).serve(service);
            let ctrlc = async {
                let message = "failed to install CTRL+C signal handler";
                tokio::signal::ctrl_c().await.expect(message);
            };

            println!("Server listening at http://{}/", address);
            Ok(server.with_graceful_shutdown(ctrlc).await?)
        })
    }
}
