use std::sync::Arc;

use crate::middleware::Middleware;

pub type Router<State> = via_router::Router<Arc<dyn Middleware<State>>>;

pub struct Route<'a, State> {
    pub(super) inner: via_router::RouteMut<'a, Arc<dyn Middleware<State>>>,
}

impl<State> Route<'_, State> {
    pub fn middleware<T>(&mut self, middleware: T)
    where
        T: Middleware<State> + 'static,
    {
        self.inner.middleware(Arc::new(middleware));
    }

    pub fn respond<T>(&mut self, middleware: T)
    where
        T: Middleware<State> + 'static,
    {
        self.inner.respond(Arc::new(middleware));
    }

    pub fn route(&mut self, path: &'static str) -> Route<'_, State> {
        Route {
            inner: self.inner.route(path),
        }
    }

    pub fn scope(mut self, scope: impl FnOnce(&mut Self)) {
        scope(&mut self);
    }
}
