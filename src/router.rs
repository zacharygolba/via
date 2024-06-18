use std::{collections::VecDeque, sync::Arc};

use crate::{
    middleware::DynMiddleware,
    request::{IncomingRequest, PathParams},
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
        Self {
            value: via_router::Router::new(),
        }
    }

    pub fn at<'a>(&'a mut self, pattern: &'static str) -> Endpoint<'a> {
        Endpoint {
            value: self.value.at(pattern),
        }
    }

    pub fn visit(&self, request: &IncomingRequest, params: &mut PathParams) -> Next {
        let mut stack = VecDeque::new();
        let path = request.uri().path();

        for matched in self.value.visit(path) {
            if let Some((name, range)) = matched.param() {
                params.insert(name, range);
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
    pub fn at(&'a mut self, pattern: &'static str) -> Self {
        Self {
            value: self.value.at(pattern),
        }
    }

    pub fn scope<T>(&mut self, scope: T) -> &mut Self
    where
        T: FnOnce(&mut Self),
    {
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
