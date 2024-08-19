use std::{collections::VecDeque, sync::Arc};

use crate::{Middleware, Next, Request};

pub struct Endpoint<'a, State> {
    inner: via_router::Endpoint<'a, Vec<MatchWhen<State>>>,
}

pub struct Router<State> {
    inner: via_router::Router<Vec<MatchWhen<State>>>,
}

/// An enum that wraps middleware before it's added to the router, specifying
/// whether the middleware should apply to partial or exact matches of a
/// request's url path.
enum MatchWhen<State> {
    /// Apply the middleware to exact matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `respond` method.
    Exact(Arc<dyn Middleware<State>>),

    /// Apply the middleware to partial matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `include` method.
    Partial(Arc<dyn Middleware<State>>),
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
        let middleware = Arc::new(middleware);

        self.route_mut().push(MatchWhen::Partial(middleware));
        self
    }

    pub fn respond<T>(&mut self, responder: T) -> &mut Self
    where
        T: Middleware<State>,
    {
        let responder = Arc::new(responder);

        self.route_mut().push(MatchWhen::Exact(responder));
        self
    }

    fn route_mut(&mut self) -> &mut Vec<MatchWhen<State>> {
        self.inner.get_or_insert_route_with(|| Box::new(Vec::new()))
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

    pub fn respond_to(&self, request: &mut Request<State>) -> Next<State> {
        let (params, path) = request.params_mut_with_path();
        let mut stack = VecDeque::with_capacity(32);

        // Iterate over the routes that match the request's path.
        for matched in self.inner.visit(path) {
            // Extend `params` with the `matched.param()` if it is `Some`.
            params.extend(matched.param());

            // Extend `stack` with middleware in `matched` depending on whether
            // or not the middleware expects a partial or exact match.
            stack.extend(matched.iter().filter_map(|when| match when {
                // Include this middleware in `stack` because it expects an exact
                // match and `matched.exact` is `true`.
                MatchWhen::Exact(exact) if matched.exact => Some(Arc::clone(exact)),

                // Include this middleware in `stack` unconditionally because it
                // targets partial matches.
                MatchWhen::Partial(partial) => Some(Arc::clone(partial)),

                // Exclude this middleware from `stack` because it expects an
                // exact match and `matched.exact` is `false`.
                MatchWhen::Exact(_) => None,
            }));
        }

        Next::new(stack)
    }
}
