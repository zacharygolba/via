pub mod middleware;
pub mod response;
pub mod routing;

pub use error::{bail, Error};
pub use http;
pub use verbs;

#[doc(inline)]
pub use self::{
    middleware::{Context, Middleware, Next},
    response::Respond,
};

use self::{response::Response, routing::*, verbs::Verb};
use radr::Label;

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

pub struct Application {
    router: radr::Router<Route>,
}

pub fn new() -> Application {
    Application {
        router: Default::default(),
    }
}

impl Application {
    pub fn at(&mut self, pattern: &'static str) -> Router {
        Router {
            root: self.router.at(pattern),
        }
    }

    pub fn call(&self, mut context: Context) -> BoxFuture<Result> {
        let (parameters, method, path) = Context::locate(&mut context);
        let matches = self.router.visit(path).flat_map(|matched| {
            let verbs = matched.verbs.get(match matched.label {
                Label::CatchAll(_) => method.into(),
                _ if matched.exact => method.into(),
                _ => Verb::none(),
            });

            match matched.param {
                Some(("", _)) | Some((_, "")) | None => {}
                Some((name, value)) => {
                    parameters.insert(name, value.to_owned());
                }
            }

            matched.stack.iter().chain(verbs)
        });

        Next::new(matches).call(context)
    }
}
