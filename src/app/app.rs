use super::router::{Route, Router};
use crate::middleware::Middleware;

pub struct App<T = ()> {
    pub(crate) state: T,
    pub(crate) router: Router<T>,
}

/// Constructs a new [`App`] with the provided `state` argument.
///
pub fn app<T>(state: T) -> App<T> {
    App {
        state,
        router: Router::new(),
    }
}

impl App {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T> App<T> {
    pub fn with_state(state: T) -> Self {
        Self {
            state,
            router: Router::new(),
        }
    }

    pub fn at(&mut self, path: &'static str) -> Route<T> {
        Route {
            inner: self.router.at(path),
        }
    }

    pub fn include(&mut self, middleware: impl Middleware<T> + 'static) {
        self.at("/").include(middleware);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::with_state(())
    }
}
