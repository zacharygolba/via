use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use super::route::{MatchWhen, Route};
use crate::middleware::Next;
use crate::request::PathParams;

#[derive(Debug)]
pub struct RouterError;

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

    #[inline]
    pub fn visit(&self, path: &str) -> Result<(PathParams, Next<T>), RouterError> {
        let mut params = Vec::with_capacity(8);
        let mut stack = Vec::with_capacity(8);

        // Iterate over the routes that match the request's path.
        for matching in self.inner.visit(path).into_iter().rev() {
            let found = match self.inner.resolve(matching) {
                Some(resolved) => resolved,
                None => return Err(RouterError),
            };

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

            // Extend `stack` with middleware in `matched` depending on whether
            // or not the middleware expects a partial or exact match.
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
                stack.push(Arc::downgrade(middleware));
            }
        }

        Ok((PathParams::new(params), Next::new(stack)))
    }
}

impl std::error::Error for RouterError {}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "an error occured when routing the request.")
    }
}
