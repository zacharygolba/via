use std::{collections::VecDeque, sync::Arc};
use via_router::{Endpoint as GenericEndpoint, Router as GenericRouter};

use crate::{
    middleware::DynMiddleware,
    request::{IncomingRequest, PathParams},
    Middleware, Next,
};

pub struct Router {
    inner: GenericRouter<Route>,
}

pub struct Endpoint<'a> {
    inner: GenericEndpoint<'a, Route>,
}

struct Route {
    middleware: Vec<DynMiddleware>,
    responders: Vec<DynMiddleware>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            inner: via_router::Router::new(),
        }
    }

    pub fn at(&mut self, pattern: &'static str) -> Endpoint {
        Endpoint {
            inner: self.inner.at(pattern),
        }
    }

    pub fn visit(&self, path_params: &mut PathParams, request: &IncomingRequest) -> Next {
        let mut stack = VecDeque::new();
        let path = request.uri().path();

        for matched in self.inner.visit(path) {
            if let Some((name, range)) = matched.param() {
                path_params.insert(name, range);
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
            inner: self.inner.at(pattern),
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
        self.inner.param()
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
        self.inner.route_mut().get_or_insert_with(|| Route {
            middleware: Vec::new(),
            responders: Vec::new(),
        })
    }
}
