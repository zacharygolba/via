use std::sync::Arc;
use via_router::VisitError;

use super::route::{MatchWhen, Route};
use crate::middleware::Next;
use crate::request::PathParams;

pub struct Router<T> {
    inner: via_router::Router<Vec<MatchWhen<T>>>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            inner: via_router::Router::new(),
        }
    }

    pub fn at(&mut self, pattern: &'static str) -> Route<T> {
        Route::new(self.inner.at(pattern))
    }

    pub fn lookup(&self, path: &str, params: &mut PathParams) -> Result<Next<T>, VisitError> {
        let mut next = Next::new(Vec::new());

        // Iterate over the routes that match the request's path.
        for matching in self.inner.visit(path).into_iter().rev() {
            let found = self.inner.resolve(matching)?;

            // If there is a dynamic parameter name associated with the route,
            // build a tuple containing the name and the range of the parameter
            // value in the request's path.
            if let Some(name) = found.param {
                params.push((name.clone(), found.range));
            }

            let route = match found.route {
                Some(route) => route,
                None => continue,
            };

            for middleware in route.iter().rev().filter_map(|when| match when {
                // Include this middleware in `stack` because it expects an exact
                // match and the visited node is considered a leaf in this
                // context.
                MatchWhen::Exact(exact) if found.exact => Some(exact),

                // Include this middleware in `stack` unconditionally because it
                // targets partial matches.
                MatchWhen::Partial(partial) => Some(partial),

                // Exclude this middleware from `stack` because it expects an
                // exact match and the visited node is not a leaf.
                MatchWhen::Exact(_) => None,
            }) {
                next.push(Arc::clone(middleware));
            }
        }

        Ok(next)
    }
}
