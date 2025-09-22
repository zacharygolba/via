use std::sync::Arc;

use super::router::{Route, Router};
use crate::middleware::Middleware;

pub struct App<State> {
    pub(super) state: Arc<State>,
    pub(super) router: Router<State>,
}

impl<State> App<State> {
    /// Constructs a new [`App`] with the provided `state` argument.
    ///
    pub fn new(state: State) -> Self {
        App {
            state: Arc::new(state),
            router: Router::new(),
        }
    }

    pub fn at(&mut self, path: &'static str) -> Route<'_, State> {
        Route {
            inner: self.router.at(path),
        }
    }

    pub fn include(&mut self, middleware: impl Middleware<State> + 'static) {
        self.at("/").include(middleware);
    }
}
