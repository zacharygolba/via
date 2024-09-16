use std::sync::Arc;

use crate::router::{Endpoint, Router};
use crate::Middleware;

pub struct App<T> {
    state: Arc<T>,
    router: Router<T>,
}

/// Constructs a new `App` with the provided `state`.
pub fn new<T: Send + Sync + 'static>(state: T) -> App<T> {
    App {
        state: Arc::new(state),
        router: Router::new(),
    }
}

impl<T: Send + Sync + 'static> App<T> {
    pub fn at(&mut self, pattern: &'static str) -> Endpoint<T> {
        self.router.at(pattern)
    }

    pub fn include(&mut self, middleware: impl Middleware<T> + 'static) -> &mut Self {
        self.at("/").include(middleware);
        self
    }
}

impl<T: Send + Sync + 'static> App<T> {
    pub(crate) fn into_parts(self) -> (Arc<T>, Router<T>) {
        (self.state, self.router)
    }
}
