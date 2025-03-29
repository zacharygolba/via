use std::sync::Arc;

use via_router::Binding;

use super::router::{Route, Router};
use crate::middleware::Middleware;

pub struct App<T> {
    pub(crate) state: Arc<T>,
    router: Router<T>,
}

/// Constructs a new [`App`] with the provided `state` argument.
///
pub fn app<T>(state: T) -> App<T> {
    App {
        state: Arc::new(state),
        router: Router::new(),
    }
}

impl<T> App<T> {
    pub fn at(&mut self, path: &'static str) -> Route<T> {
        Route {
            inner: self.router.at(path),
        }
    }

    pub fn include(&mut self, middleware: impl Middleware<T> + 'static) {
        self.at("/").include(middleware);
    }
}

impl<T> App<T> {
    pub(crate) fn visit(&self, path: &str) -> Vec<Binding<Arc<dyn Middleware<T>>>> {
        self.router.visit(path)
    }
}
