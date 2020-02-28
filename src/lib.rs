mod failure;
mod handler;
mod runtime;

pub mod context;
pub mod helpers;
pub mod prelude;
pub mod respond;
pub mod routing;

use self::{respond::Response, routing::*, runtime::MakeService};
use http::Extensions;
use hyper::Server;
use std::pin::Pin;

#[doc(inline)]
pub use self::{context::Context, failure::Error, handler::*, respond::Respond};
pub use codegen::*;
pub use http;

#[doc(hidden)]
pub type Future = Pin<Box<dyn std::future::Future<Output = Result> + Send>>;
#[doc(hidden)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

#[derive(Default)]
pub struct Application {
    router: Router,
    state: Extensions,
}

#[macro_export]
macro_rules! json {
    { $($tokens:tt)+ } => {
        $crate::respond::json(&serde_json::json!({ $($tokens)+ }))
    };
    ($status:expr, { $($tokens:tt)+ }) => {
        ($crate::json! { $($tokens)+ }).status($status)
    };
}

#[macro_export]
macro_rules! thunk {
    { $(const $name:ident = $expr:expr;)* } => {
        $(fn $name() -> impl $crate::Handler {
            $expr
        })*
    }
}

#[inline]
pub fn new() -> Application {
    Default::default()
}

impl Application {
    #[inline]
    pub fn at(&mut self, path: &'static str) -> Location {
        Location {
            value: self.router.at(path),
        }
    }

    #[inline]
    pub fn inject(&mut self, value: impl Send + Sync + 'static) {
        self.state.insert(value);
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
