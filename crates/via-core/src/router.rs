use crate::{middleware::DynMiddleware, Context, Middleware, Next};
use std::sync::Arc;
use verbs::Map;

pub use verbs::Verb;

pub type Location<'a> = radr::Location<'a, Route>;

pub trait Service: Send + Sync + 'static {
    fn mount(self: Arc<Self>, to: &mut Location);
}

pub trait Target {
    fn mount<T: Service>(&mut self, service: T);
}

#[derive(Default)]
pub struct Route {
    verbs: Map<DynMiddleware>,
    stack: Vec<DynMiddleware>,
}

#[derive(Default)]
pub struct Router {
    value: radr::Router<Route>,
}

impl<'a> Target for Location<'a> {
    fn mount<T: Service>(&mut self, service: T) {
        Service::mount(Arc::new(service), self);
    }
}

impl Route {
    pub fn expose(&mut self, verb: Verb, action: impl Middleware) {
        self.verbs.insert(verb, Arc::new(action));
    }

    pub fn include(&mut self, middleware: impl Middleware) {
        self.stack.push(Arc::new(middleware));
    }
}

impl Router {
    pub fn at(&mut self, pattern: &'static str) -> Location {
        self.value.at(pattern)
    }

    pub fn visit(&self, context: &mut Context) -> Next {
        let (parameters, method, path) = context.locate();

        Next::new(self.value.visit(path).flat_map(|route| {
            let verbs = route.verbs.get(match route.label {
                radr::Label::CatchAll(_) => method.into(),
                _ if route.exact => method.into(),
                _ => Verb::none(),
            });

            match route.param {
                Some(("", _)) | Some((_, "")) | None => {}
                Some((name, value)) => {
                    parameters.insert(name, value.to_owned());
                }
            }

            route.stack.iter().chain(verbs)
        }))
    }
}
