use router::{Endpoint as GenericEndpoint, Router as GenericRouter};
use std::{collections::VecDeque, sync::Arc};

use crate::{
    middleware::{context::PathParams, DynMiddleware},
    HttpRequest, Middleware, Next,
};

pub struct Router {
    value: GenericRouter<Route>,
}

pub struct Endpoint<'a> {
    value: GenericEndpoint<'a, Route>,
}

struct Route {
    middleware: Vec<DynMiddleware>,
    responders: Vec<DynMiddleware>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            value: router::Router::new(),
        }
    }

    pub fn at(&mut self, pattern: &'static str) -> Endpoint {
        Endpoint {
            value: self.value.at(pattern),
        }
    }

    pub fn visit(&self, request: &HttpRequest, params: &mut PathParams) -> Next {
        let mut stack = VecDeque::new();

        if let Some(route) = self.value.route() {
            stack.extend(route.middleware.iter().cloned());
        }

        for matched in self.value.visit(request.uri().path()) {
            if let Some((name, value)) = matched.param() {
                params.insert(name, value);
            }

            if let Some(route) = matched.route() {
                stack.extend(route.middleware.iter().cloned());
                if matched.is_exact_match {
                    stack.extend(route.responders.iter().cloned());
                }
            }
        }

        Next::new(stack)
    }
}

impl<'a> Endpoint<'a> {
    pub fn at(&mut self, pattern: &'static str) -> Endpoint {
        Endpoint {
            value: self.value.at(pattern),
        }
    }

    pub fn scope(&mut self, scope: impl FnOnce(&mut Self)) -> &mut Self {
        scope(self);
        self
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
