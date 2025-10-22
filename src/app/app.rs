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

    /// Append the provided middleware to applications call stack.
    ///
    /// Middleware attached to the root path `/` runs unconditionally for every
    /// request.
    ///
    /// See [`Route::middleware`] for additional usage docs.
    ///
    pub fn middleware<T>(&mut self, middleware: T)
    where
        T: Middleware<State> + 'static,
    {
        self.route("/").middleware(middleware);
    }

    /// Returns a new route as a child of the root path `/`.
    ///
    /// See [`Route::route`] for additional usage docs.
    ///
    pub fn route(&mut self, path: &'static str) -> Route<'_, State> {
        Route {
            inner: self.router.route(path),
        }
    }
}
