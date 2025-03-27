use std::sync::Arc;

use crate::middleware::Middleware;
use crate::router::{Route, Router};

pub struct App<T> {
    pub(crate) state: Arc<T>,
    pub(crate) router: via_router::Builder<Box<dyn Middleware<T>>>,
}

/// Constructs a new [`App`] with the provided `state` argument.
///
pub fn app<T>(state: T) -> App<T> {
    App {
        state: Arc::new(state),
        router: Router::build(),
    }
}

impl<T> App<T> {
    pub fn at(&mut self, pattern: &'static str) -> Route<T> {
        self.router.at(pattern).into()
    }

    pub fn include(&mut self, middleware: impl Middleware<T> + 'static) -> &mut Self {
        self.at("/").include(middleware);
        self
    }
}
