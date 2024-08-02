use std::{collections::VecDeque, sync::Arc};

use crate::{middleware::ArcMiddleware, Middleware, Next, Request};

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

    pub fn visit(&self, request: &mut Request<State>) -> Next<State> {
        let mut stack = VecDeque::with_capacity(48);
        let matches = self.inner.visit(request.uri().path());
        let params = request.params_mut();

        for matched in matches {
            if let Some((name, value)) = matched.param() {
                params.push((Arc::clone(name), value));
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

    pub fn param(&self) -> Option<&Arc<str>> {
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
