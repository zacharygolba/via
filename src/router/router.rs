use std::sync::Arc;

use crate::middleware::Middleware;
use via_router::MatchCond;

pub(crate) type Router<T> = via_router::Router<MatchCond<Arc<dyn Middleware<T>>>>;

type Inner<'a, T> = via_router::Route<'a, MatchCond<Arc<dyn Middleware<T>>>>;

pub struct Route<'a, T> {
    inner: Inner<'a, T>,
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
        self.inner.push(MatchCond::Partial(Arc::new(middleware)));
    }

    pub fn respond<M>(&mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner.push(MatchCond::Exact(Arc::new(middleware)));
    }
}

impl<'a, T> From<Inner<'a, T>> for Route<'a, T> {
    fn from(inner: Inner<'a, T>) -> Self {
        Self { inner }
    }
}
