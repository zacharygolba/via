use std::{collections::VecDeque, sync::Arc};

use crate::middleware::{ArcMiddleware, BoxFuture};
use crate::request::PathParams;
use crate::{Error, Middleware, Next, Request, Response};

pub struct Router<State> {
    inner: via_router::Router<Route<State>>,
}

pub struct Endpoint<'a, State> {
    inner: via_router::Endpoint<'a, Route<State>>,
}

struct Route<State> {
    middleware: Vec<ArcMiddleware<State>>,
    responders: Vec<ArcMiddleware<State>>,
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

    pub fn respond_to(&self, mut request: Request<State>) -> BoxFuture<Result<Response, Error>> {
        let (params, path) = request.params_mut_with_path();
        let next = self.visit(params, path);

        next.call(request)
    }

    fn visit(&self, params: &mut PathParams, path: &str) -> Next<State> {
        let mut stack = VecDeque::new();
        let matches = self.inner.visit(path);

        for matched in matches {
            if let Some(param) = matched.param() {
                params.push(param);
            }

            if let Some(route) = matched.route {
                for middleware in &route.middleware {
                    stack.push_back(Arc::clone(middleware));
                }

                if matched.exact {
                    for responder in &route.responders {
                        stack.push_back(Arc::clone(responder));
                    }
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
        self.inner.get_or_insert_route_with(|| {
            Box::new(Route {
                middleware: Vec::new(),
                responders: Vec::new(),
            })
        })
    }
}
