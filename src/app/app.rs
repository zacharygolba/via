use std::sync::Arc;

use super::router::{Route, Router};
use crate::middleware::Middleware;

pub struct App<State> {
    pub(super) state: Arc<State>,
    pub(super) router: Router<State>,
}

/// Constructs a new [`App`] with the provided `state` argument.
///
pub fn app<State>(state: State) -> App<State> {
    App {
        state: Arc::new(state),
        router: Router::new(),
    }
}

impl<State> App<State> {
    pub fn at(&mut self, path: &'static str) -> Route<'_, State> {
        Route {
            inner: self.router.at(path),
        }
    }

    pub fn include(&mut self, middleware: impl Middleware<State> + 'static) {
        self.at("/").include(middleware);
    }
}
