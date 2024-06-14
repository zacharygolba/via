use std::{collections::VecDeque, sync::Arc};

use crate::{
    middleware::DynMiddleware,
    request::{HyperRequest, PathParams},
    Middleware, Next,
};

pub struct Router {
    value: via_router::Router<Route>,
}

pub struct Endpoint<'a> {
    value: via_router::Endpoint<'a, Route>,
}

struct Route {
    middleware: Vec<DynMiddleware>,
    responders: Vec<DynMiddleware>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            value: via_router::Router::new(),
        }
    }

    pub fn at(&mut self, pattern: &'static str) -> Endpoint {
        Endpoint {
            value: self.value.at(pattern),
        }
    }

    pub fn visit(&self, request: &HyperRequest, params: &mut PathParams) -> Next {
        let mut stack = VecDeque::new();

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

    pub fn include<T>(&mut self, middleware: T) -> &mut Self
    where
        T: Middleware,
    {
        let route = self.route_mut();

        route.middleware.push(Arc::pin(middleware));
        self
    }

    pub fn respond<T>(&mut self, responder: T) -> &mut Self
    where
        T: Middleware,
    {
        let route = self.route_mut();

        route.responders.push(Arc::pin(responder));
        self
    }

    fn route_mut(&mut self) -> &mut Route {
        self.value.route_mut().get_or_insert_with(|| Route {
            middleware: Vec::new(),
            responders: Vec::new(),
        })
    }
}
