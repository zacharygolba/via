use router::{Router as GenericRouter, Verb};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use crate::{middleware::DynMiddleware, Context, Middleware, Next};

pub type Location<'a> = router::Location<'a, Route>;

pub trait Service: Send + Sync + 'static {
    fn connect(self: Arc<Self>, to: &mut Location);
}

pub trait Endpoint {
    fn delegate<T: Service>(&mut self, service: T);
}

#[derive(Default, Debug)]
pub struct Router(GenericRouter<Route>);

#[derive(Default)]
pub struct Route {
    middleware: Vec<DynMiddleware>,
    responders: Vec<DynMiddleware>,
}

impl<'a> Endpoint for Location<'a> {
    fn delegate<T: Service>(&mut self, service: T) {
        Service::connect(Arc::new(service), self);
    }
}

impl Route {
    pub fn connect(&mut self, middleware: impl Middleware) {
        self.handle(Verb::CONNECT, middleware);
    }

    pub fn delete(&mut self, middleware: impl Middleware) {
        self.handle(Verb::DELETE, middleware);
    }

    pub fn get(&mut self, middleware: impl Middleware) {
        self.handle(Verb::GET, middleware);
    }

    pub fn head(&mut self, middleware: impl Middleware) {
        self.handle(Verb::HEAD, middleware);
    }

    pub fn options(&mut self, middleware: impl Middleware) {
        self.handle(Verb::OPTIONS, middleware);
    }

    pub fn patch(&mut self, middleware: impl Middleware) {
        self.handle(Verb::PATCH, middleware);
    }

    pub fn post(&mut self, middleware: impl Middleware) {
        self.handle(Verb::POST, middleware);
    }

    pub fn put(&mut self, middleware: impl Middleware) {
        self.handle(Verb::PUT, middleware);
    }

    pub fn trace(&mut self, middleware: impl Middleware) {
        self.handle(Verb::TRACE, middleware);
    }

    pub fn handle(&mut self, verb: Verb, responder: impl Middleware) {
        let responder = Arc::new(move |context: Context, next: Next| {
            if verb.intersects(context.method().into()) {
                responder.call(context, next)
            } else {
                next.call(context)
            }
        });

        self.responders.push(responder);
    }

    pub fn include(&mut self, middleware: impl Middleware) -> &mut Self {
        self.middleware.push(Arc::new(middleware));
        self
    }
}

impl Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Route")
    }
}

impl Router {
    pub fn at(&mut self, pattern: &'static str) -> Location {
        self.0.at(pattern)
    }

    pub fn visit(&self, context: &mut Context) -> Next {
        let (parameters, _, path) = context.locate();
        let middleware = self.0.middleware.iter();

        Next::new(middleware.chain(self.0.visit(path).flat_map(|matched| {
            if let Some((name, value)) = matched.param {
                parameters.insert(name, value.to_owned());
            }

            matched.route.middleware.iter().chain(if matched.is_exact {
                &matched.route.responders[..]
            } else {
                &[]
            })
        })))
    }
}
