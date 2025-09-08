use std::sync::Arc;

use crate::middleware::Middleware;

pub type Router<T> = via_router::Router<Arc<dyn Middleware<T>>>;

pub struct Scope<'a, T> {
    pub(super) inner: via_router::RouteMut<'a, Arc<dyn Middleware<T>>>,
}

impl<T> Scope<'_, T> {
    pub fn at(&mut self, path: &'static str) -> Scope<'_, T> {
        Scope {
            inner: self.inner.at(path),
        }
    }

    pub fn scope(&mut self, scope: impl FnOnce(&mut Self)) {
        scope(self);
    }

    pub fn include<M>(&mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner.include(Arc::new(middleware));
    }

    pub fn respond<M>(&mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner.respond(Arc::new(middleware));
    }
}
