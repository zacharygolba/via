use router::{Location as GenericLocation, Router as GenericRouter};
use std::{collections::VecDeque, sync::Arc};

use crate::{middleware::DynMiddleware, Context, Middleware, Next};

pub trait Service: Send + Sync + 'static {
    fn connect(self: Arc<Self>, to: &mut Location);
}

pub trait Endpoint {
    fn delegate<T: Service>(&mut self, service: T);
}

pub struct Router {
    value: GenericRouter<Route>,
}

pub struct Location<'a> {
    value: GenericLocation<'a, Route>,
}

pub struct Route {
    middleware: Vec<DynMiddleware>,
    responders: Vec<DynMiddleware>,
}

impl<'a> Endpoint for Location<'a> {
    fn delegate<T: Service>(&mut self, service: T) {
        Service::connect(Arc::new(service), self);
    }
}

impl Router {
    pub fn new() -> Self {
        Router {
            value: router::Router::new(),
        }
    }

    pub fn at(&mut self, pattern: &'static str) -> Location {
        Location {
            value: self.value.at(pattern),
        }
    }

    pub fn visit(&self, context: &mut Context) -> Next {
        let (parameters, _, path) = context.locate();
        let mut stack = VecDeque::new();

        if let Some(root) = self.value.root() {
            stack.extend(root.middleware.iter().cloned());
        }

        for matched in self.value.visit(path) {
            if let Some((name, value)) = matched.param() {
                parameters.insert(name, value.to_owned());
            }

            if let Some(route) = matched.route() {
                stack.extend(route.middleware.iter().cloned());
                stack.extend(route.responders.iter().cloned());
            }
        }

        Next::new(stack)
    }
}

impl<'a> Location<'a> {
    pub fn at(&'a mut self, pattern: &'static str) -> Self {
        Location {
            value: self.value.at(pattern),
        }
    }

    pub fn param(&self) -> Option<&'static str> {
        self.value.param()
    }

    pub fn include(&mut self, middleware: impl Middleware) -> &mut Self {
        let route = self.route_mut();

        route.middleware.push(Arc::new(middleware));
        self
    }

    pub fn respond(&mut self, responder: impl Middleware) -> &mut Self {
        let route = self.route_mut();

        route.responders.push(Arc::new(responder));
        self
    }

    fn route_mut(&mut self) -> &mut Route {
        self.value.route_mut().get_or_insert_with(|| Route {
            middleware: Vec::new(),
            responders: Vec::new(),
        })
    }
}
