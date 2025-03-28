use std::sync::Arc;
use via_router::MatchCond;

use crate::middleware::Middleware;

pub(crate) type Router<T> = via_router::Router<MatchCond<Arc<dyn Middleware<T>>>>;

pub struct Route<'a, T> {
    pub(crate) inner: via_router::Route<'a, MatchCond<Arc<dyn Middleware<T>>>>,
}

impl<T> Route<'_, T> {
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
        self.inner.push(MatchCond::Partial(Arc::new(middleware)));
    }

    pub fn respond<M>(&mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner.push(MatchCond::Exact(Arc::new(middleware)));
    }
}
