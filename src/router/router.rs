use std::sync::Arc;

use crate::middleware::Middleware;
use via_router::MatchCond;

pub(crate) type Router<T> = via_router::Router<Vec<MatchWhen<T>>>;

/// An enum that wraps middleware before it's added to the router, specifying
/// whether the middleware should apply to partial or exact matches of a
/// request's url path.
///
pub enum MatchWhen<T> {
    /// Apply the middleware to exact matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `respond` method.
    ///
    Exact(Box<dyn Middleware<T>>),

    /// Apply the middleware to partial matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `include` method.
    ///
    Partial(Box<dyn Middleware<T>>),
}

pub struct Route<'a, T> {
    inner: via_router::Route<'a, Vec<Arc<dyn Middleware<T>>>>,
}

impl<'a, T> Route<'a, T> {
    pub fn at(&mut self, pattern: &'static str) -> Route<T> {
        Route {
            inner: self.inner.at(pattern),
        }
    }

    pub fn scope<F>(&mut self, scope: F)
    where
        F: FnOnce(&mut Self),
    {
        scope(self);
    }

    pub fn include<M>(&mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner
            .as_mut()
            .get_or_insert_default()
            .push(MatchCond::Partial(Arc::new(middleware)));
    }

    pub fn respond<M>(mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner.push(MatchCond::Exact(Box::new(middleware)));
    }
}

impl<'a, T> From<via_router::Route<'a, Box<dyn Middleware<T>>>> for Route<'a, T> {
    fn from(inner: via_router::Route<'a, Box<dyn Middleware<T>>>) -> Self {
        Self { inner }
    }
}
