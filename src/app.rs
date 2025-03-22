use crate::middleware::Middleware;
use crate::router::{Route, Router};

pub struct App<T> {
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

impl<T> App<T> {
    pub fn at(&mut self, pattern: &'static str) -> Route<T> {
        self.router.at(pattern).into()
    }

    pub fn include(&mut self, middleware: impl Middleware<T> + 'static) -> &mut Self {
        self.at("/").include(middleware);
        self
    }
}
