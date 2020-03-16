use crate::{ArcMiddleware, BoxFuture, Context, Middleware, Next, Result, State};
use radr::{Label, Location};
use std::sync::Arc;
use verbs::*;

pub trait Service: Send + Sync + 'static {
    #[doc(hidden)]
    fn mount(self: Arc<Self>, router: &mut Router);
}

#[doc(hidden)]
pub struct Router<'a> {
    state: &'a mut State,
    value: Location<'a, Endpoint>,
}

#[derive(Default)]
pub(crate) struct Routes {
    router: radr::Router<Endpoint>,
}

#[derive(Default)]
struct Endpoint {
    verbs: Map<ArcMiddleware>,
    stack: Vec<ArcMiddleware>,
}

impl<'a> Router<'a> {
    #[doc(hidden)]
    pub fn expose(&mut self, verb: Verb, middleware: impl Middleware) {
        self.value.verbs.insert(verb, Arc::new(middleware));
    }

    pub fn middleware(&mut self, middleware: impl Middleware) {
        self.value.stack.push(Arc::new(middleware));
    }

    #[doc(hidden)]
    pub fn namespace(&mut self, pattern: &'static str) -> Router {
        Router {
            state: self.state,
            value: self.value.at(pattern),
        }
    }

    pub fn service(&mut self, service: impl Service) {
        Service::mount(Arc::new(service), self);
    }
}

impl Routes {
    #[doc(hidden)]
    pub fn namespace<'a>(&'a mut self, state: &'a mut State, pattern: &'static str) -> Router<'a> {
        Router {
            state,
            value: self.router.at(pattern),
        }
    }

    #[inline]
    pub fn visit(&self, mut context: Context) -> BoxFuture<Result> {
        let (parameters, method, path) = context.locate();
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
