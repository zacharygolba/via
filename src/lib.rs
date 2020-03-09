mod error;
mod handler;
mod routing;
mod runtime;
mod server;

pub mod helpers;
pub mod prelude;

use self::{handler::Request, http::Extensions, verbs::Verb};
use std::sync::Arc;

pub use self::{error::*, handler::*, routing::*};
pub use codegen::*;
pub use http;
pub use verbs;

#[derive(Default)]
pub struct App {
    router: Router,
    state: Arc<Extensions>,
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
            state: Arc::get_mut(&mut self.state).unwrap(),
            value: self.router.at(path),
        }
    }

    #[inline]
    pub fn call(&self, request: Request) -> Future {
        let mut context = Context::new(self.state.clone(), request);
        let parameters = &mut context.parameters;
        let method = context.request.method();
        let route = context.request.uri().path();
        let next = Next::new(self.router.visit(route).flat_map(|matched| {
            let verbs = matched.verbs.get(if matched.exact {
                method.into()
            } else {
                Verb::none()
            });

            match matched.param {
                Some(("", _)) | Some((_, "")) | None => {}
                Some((name, value)) => {
                    parameters.insert(name, value.to_owned());
                }
            }

            matched.stack.iter().chain(verbs)
        }));

        next.call(context)
    }

    #[inline]
    pub fn inject(&mut self, value: impl Send + Sync + 'static) {
        Arc::get_mut(&mut self.state).unwrap().insert(value);
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
