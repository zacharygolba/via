use std::sync::Arc;

use via_router::VisitError;

use crate::middleware::Middleware;
use crate::request::PathParams;
use crate::Next;

/// An enum that wraps middleware before it's added to the router, specifying
/// whether the middleware should apply to partial or exact matches of a
/// request's url path.
pub enum MatchWhen<State> {
    /// Apply the middleware to exact matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `respond` method.
    Exact(Arc<dyn Middleware<State>>),

    /// Apply the middleware to partial matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `include` method.
    Partial(Arc<dyn Middleware<State>>),
}

pub struct Endpoint<'a, State> {
    inner: via_router::Endpoint<'a, Vec<MatchWhen<State>>>,
}

pub struct Router<State> {
    inner: via_router::Router<Vec<MatchWhen<State>>>,
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

    pub fn param(&self) -> Option<&str> {
        self.inner.param().map(|name| name.as_str())
    }

    pub fn include<T>(&mut self, middleware: T) -> &mut Self
    where
        T: Middleware<State> + 'static,
    {
        let middleware = Arc::new(middleware);

        self.route_mut().push(MatchWhen::Partial(middleware));
        self
    }

    pub fn respond<T>(&mut self, responder: T) -> &mut Self
    where
        T: Middleware<State> + 'static,
    {
        let responder = Arc::new(responder);

        self.route_mut().push(MatchWhen::Exact(responder));
        self
    }

    fn route_mut(&mut self) -> &mut Vec<MatchWhen<State>> {
        self.inner.get_or_insert_route_with(Vec::new)
    }
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

    pub fn lookup(&self, path: &str, params: &mut PathParams) -> Result<Next<State>, VisitError> {
        let mut stack = Vec::new();

        // Iterate over the routes that match the request's path.
        for result in self.inner.visit(path).into_iter().rev() {
            let found = result?;

            // If there is a dynamic parameter name associated with the route,
            // build a tuple containing the name and the range of the parameter
            // value in the request's path.
            if let (Some(param), Some(at)) = (found.param, found.at) {
                params.push((param, at));
            }

            let route = match found.route.and_then(|key| self.inner.get(key)) {
                Some(route) => route,
                None => continue,
            };

            // Extend `stack` with middleware in `matched` depending on whether
            // or not the middleware expects a partial or exact match.
            for middleware in route.iter().rev().filter_map(|when| match when {
                // Include this middleware in `stack` because it expects an exact
                // match and the visited node is considered a leaf in this
                // context.
                MatchWhen::Exact(exact) if found.is_leaf => Some(exact),

                // Include this middleware in `stack` unconditionally because it
                // targets partial matches.
                MatchWhen::Partial(partial) => Some(partial),

                // Exclude this middleware from `stack` because it expects an
                // exact match and the visited node is not a leaf.
                MatchWhen::Exact(_) => None,
            }) {
                stack.push(Arc::clone(middleware));
            }
        }

        Ok(Next::new(stack))
    }
}
