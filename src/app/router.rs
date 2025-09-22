use std::sync::Arc;

use crate::middleware::Middleware;

pub type Router<State> = via_router::Router<Arc<dyn Middleware<State>>>;

pub struct Route<'a, State> {
    pub(super) inner: via_router::RouteMut<'a, Arc<dyn Middleware<State>>>,
}

impl<State> Route<'_, State> {
    pub fn at(&mut self, path: &'static str) -> Route<'_, State> {
        Route {
            inner: self.inner.at(path),
        }
    }

    pub fn scope(mut self, scope: impl FnOnce(&mut Self)) {
        scope(&mut self);
    }

    pub fn include(&mut self, middleware: impl Middleware<State> + 'static) {
        self.inner.include(Arc::new(middleware));
    }

    pub fn respond(&mut self, middleware: impl Middleware<State> + 'static) {
        self.inner.respond(Arc::new(middleware));
    }
}
