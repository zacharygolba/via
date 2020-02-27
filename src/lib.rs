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
}

#[macro_export]
macro_rules! sync {
    ($expr:expr) => {
        tokio::task::spawn_blocking($expr).await
    };
}

#[inline]
pub fn new() -> Application {
    Default::default()
}

#[inline]
pub async fn start(application: Application) -> Result<()> {
    let address = "0.0.0.0:8080".parse()?;
    let service = MakeService::from(application);
    let server = Server::bind(&address).serve(service);
    let ctrlc = async {
        let message = "failed to install CTRL+C signal handler";
        tokio::signal::ctrl_c().await.expect(message);
    };

    println!("Server listening at http://{}/", address);
    Ok(server.with_graceful_shutdown(ctrlc).await?)
}

impl Application {
    #[inline]
    pub fn at(&mut self, path: &'static str) -> Location {
        Location {
            value: self.router.at(path),
        }
    }

    #[inline]
    pub fn plug(&mut self, handler: impl Handler) -> &mut Self {
        self.at("/").plug(handler);
        self
    }

    #[inline]
    pub fn route(&mut self, route: impl Route) -> &mut Self {
        self.at("/").route(route);
        self
    }

    #[inline]
    pub fn state(&mut self, value: impl Send + Sync + 'static) -> &mut Self {
        self.state.insert(value);
        self
    }
}
