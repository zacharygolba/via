use std::{collections::VecDeque, sync::Arc};
use via_router::{Endpoint as GenericEndpoint, Router as GenericRouter};

use crate::{
    middleware::DynMiddleware,
    request::{self, PathParams},
    Middleware, Next,
};

pub struct Router<State> {
    inner: GenericRouter<Route<State>>,
}

pub struct Endpoint<'a, State> {
    inner: GenericEndpoint<'a, Route<State>>,
}

struct Route<State> {
    middleware: Vec<DynMiddleware<State>>,
    responders: Vec<DynMiddleware<State>>,
}

impl<State> Router<State>
where
    State: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            inner: via_router::Router::new(),
        }
    }

    pub fn at(&mut self, pattern: &'static str) -> Endpoint<State> {
        Endpoint {
            inner: self.inner.at(pattern),
        }
    }

    pub fn visit(
        &self,
        path_params: &mut PathParams,
        request: &http::Request<request::Body>,
    ) -> Next<State> {
        let mut stack = VecDeque::new();
        let path = request.uri().path();

        for matched in self.inner.visit(path) {
            if let Some(param) = matched.param() {
                path_params.push(param);
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

impl<'a, State> Endpoint<'a, State> {
    pub fn at(&mut self, pattern: &'static str) -> Endpoint<State> {
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
        T: Middleware<State>,
    {
        let route = self.route_mut();

        route.middleware.push(Arc::new(middleware));
        self
    }

    pub fn respond<T>(&mut self, responder: T) -> &mut Self
    where
        T: Middleware<State>,
    {
        let route = self.route_mut();

        route.responders.push(Arc::new(responder));
        self
    }

    fn route_mut(&mut self) -> &mut Route<State> {
        self.inner.route_mut().get_or_insert_with(|| Route {
            middleware: Vec::new(),
            responders: Vec::new(),
        })
    }
}
