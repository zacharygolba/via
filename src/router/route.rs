use std::sync::Arc;

use crate::middleware::Middleware;

/// An enum that wraps middleware before it's added to the router, specifying
/// whether the middleware should apply to partial or exact matches of a
/// request's url path.
///
pub enum MatchWhen<T> {
    /// Apply the middleware to exact matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `respond` method.
    ///
    Exact(Arc<dyn Middleware<T>>),

    /// Apply the middleware to partial matches of a request's url path. This
    /// variant is used when middleware is added to an `Endpoint` with the
    /// `include` method.
    ///
    Partial(Arc<dyn Middleware<T>>),
}

pub struct Route<'a, T> {
    inner: via_router::Route<'a, Vec<MatchWhen<T>>>,
}

impl<T> Route<'_, T> {
    pub fn at(&mut self, pattern: &'static str) -> Route<T> {
        Route {
            inner: self.inner.at(pattern),
        }
    }

    pub fn scope(&mut self, scope: impl FnOnce(&mut Self)) -> &mut Self {
        scope(self);
        self
    }

    pub fn param(&self) -> Option<&str> {
        self.inner.param()
    }

    pub fn include(&mut self, middleware: impl Middleware<T> + 'static) -> &mut Self {
        self.push(MatchWhen::Partial(Arc::new(middleware)));
        self
    }

    pub fn respond(&mut self, middleware: impl Middleware<T> + 'static) -> &mut Self {
        self.push(MatchWhen::Exact(Arc::new(middleware)));
        self
    }
}

impl<'a, T> Route<'a, T> {
    #[inline]
    pub(super) fn new(inner: via_router::Route<'a, Vec<MatchWhen<T>>>) -> Self {
        Self { inner }
    }

    fn push(&mut self, middleware: MatchWhen<T>) {
        self.inner
            .get_or_insert_route_with(Vec::new)
            .push(middleware);
    }
}
