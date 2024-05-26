use router::{verb::*, Pattern, Router as GenericRouter};
use std::sync::Arc;

use crate::{middleware::DynMiddleware, Context, Middleware, Next};

pub type Location<'a> = router::Location<'a, Route>;

pub trait Service: Send + Sync + 'static {
    fn connect(self: Arc<Self>, to: &mut Location);
}

pub trait Endpoint {
    fn delegate<T: Service>(&mut self, service: T);
}

#[derive(Default)]
pub struct Router(GenericRouter<Route>);

#[derive(Default)]
pub struct Route {
    verbs: Map<DynMiddleware>,
    stack: Vec<DynMiddleware>,
}

impl<'a> Endpoint for Location<'a> {
    fn delegate<T: Service>(&mut self, service: T) {
        Service::connect(Arc::new(service), self);
    }
}

impl Route {
    pub fn connect(&mut self, action: impl Middleware) {
        self.handle(Verb::CONNECT, action);
    }

    pub fn delete(&mut self, action: impl Middleware) {
        self.handle(Verb::DELETE, action);
    }

    pub fn get(&mut self, action: impl Middleware) {
        self.handle(Verb::GET, action);
    }

    pub fn head(&mut self, action: impl Middleware) {
        self.handle(Verb::HEAD, action);
    }

    pub fn options(&mut self, action: impl Middleware) {
        self.handle(Verb::OPTIONS, action);
    }

    pub fn patch(&mut self, action: impl Middleware) {
        self.handle(Verb::PATCH, action);
    }

    pub fn post(&mut self, action: impl Middleware) {
        self.handle(Verb::POST, action);
    }

    pub fn put(&mut self, action: impl Middleware) {
        self.handle(Verb::PUT, action);
    }

    pub fn trace(&mut self, action: impl Middleware) {
        self.handle(Verb::TRACE, action);
    }

    pub fn handle(&mut self, verb: Verb, action: impl Middleware) {
        self.verbs.insert(verb, Arc::new(action));
    }

    pub fn include(&mut self, middleware: impl Middleware) -> &mut Self {
        self.stack.push(Arc::new(middleware));
        self
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
