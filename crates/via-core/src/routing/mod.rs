use router::{verb::*, Pattern, Router as GenericRouter};
use std::sync::Arc;

use crate::{middleware::DynMiddleware, Context, Middleware, Next};

pub type Location<'a> = router::Location<'a, Route>;

pub trait Service: Send + Sync + 'static {
    fn connect(self: Arc<Self>, to: &mut Location);
}

pub trait Endpoint {
    fn connect<T: Service>(&mut self, service: T);
    fn service<T: Service>(&mut self, service: T) {
        self.connect(service)
    }
}

#[derive(Default)]
pub struct Router(GenericRouter<Route>);

#[derive(Default)]
pub struct Route {
    verbs: Map<DynMiddleware>,
    stack: Vec<DynMiddleware>,
}

impl<'a> Endpoint for Location<'a> {
    fn connect<T: Service>(&mut self, service: T) {
        Service::connect(Arc::new(service), self);
    }
}

impl Route {
    pub fn handle(&mut self, verb: Verb, action: impl Middleware) {
        self.verbs.insert(verb, Arc::new(action));
    }

    pub fn include(&mut self, middleware: impl Middleware) {
        self.stack.push(Arc::new(middleware));
    }
}

impl Router {
    pub fn at(&mut self, pattern: &'static str) -> Location {
        self.0.at(pattern)
    }

    pub fn visit(&self, context: &mut Context) -> Next {
        let (parameters, method, path) = context.locate();

        Next::new(self.0.visit(path).flat_map(|route| {
            let verbs = route.verbs.get(match route.label {
                Pattern::CatchAll(_) => method.into(),
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
