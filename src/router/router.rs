use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use via_router::Match;

use super::route::{MatchWhen, Route};
use crate::middleware::Next;
use crate::request::PathParams;

#[derive(Debug)]
pub struct RouterError {
    message: String,
}

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
    pub fn visit(&self, path: &str) -> Vec<Option<Match>> {
        self.inner.visit(path)
    }

    pub fn resolve(
        &self,
        matched: &[Option<Match>],
        path_params: &mut PathParams,
    ) -> Result<Next<T>, RouterError> {
        let mut middlewares = Vec::with_capacity(8);

        // Iterate over the routes that match the request's path.
        for option in matched.iter().rev() {
            let matching = option.as_ref().ok_or_else(RouterError::new)?;
            let (param, route) = self.inner.resolve(matching);

            // If there is a dynamic parameter name associated with the route,
            // build a tuple containing the name and the range of the parameter
            // value in the request's path.
            if let Some(name) = param {
                path_params.push(name, matching.range);
            }

            let stack = match route {
                Some(middleware) => middleware,
                None => continue,
            };

            // Extend `stack` with middleware in `matched` depending on whether
            // or not the middleware expects a partial or exact match.
            for middleware in stack.iter().rev().filter_map(|when| match when {
                // Include this middleware in `stack` because it expects an exact
                // match and the visited node is considered a leaf in this
                // context.
                MatchWhen::Exact(exact) if matching.exact => Some(exact),

                // Include this middleware in `stack` unconditionally because it
                // targets partial matches.
                MatchWhen::Partial(partial) => Some(partial),

                // Exclude this middleware from `stack` because it expects an
                // exact match and the visited node is not a leaf.
                MatchWhen::Exact(_) => None,
            }) {
                middlewares.push(Arc::clone(middleware));
            }
        }

        Ok(Next::new(middlewares))
    }
}

impl RouterError {
    fn new() -> Self {
        Self {
            message: "an error occurred when routing the request".to_owned(),
        }
    }
}

impl std::error::Error for RouterError {}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}
