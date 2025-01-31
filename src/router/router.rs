use via_router::{Found, Match, RouterError};

use super::route::{MatchWhen, Route};

pub struct Router<T> {
    inner: via_router::Router<Vec<MatchWhen<T>>>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            inner: via_router::Router::new(),
        }
    }

    pub fn at(&mut self, pattern: &'static str) -> Route<T> {
        Route::new(self.inner.at(pattern))
    }

    #[inline]
    pub fn visit(&self, path: &str) -> Vec<Match> {
        self.inner.visit(path)
    }

    #[inline]
    pub fn resolve(&self, matching: Match) -> Result<Found<Vec<MatchWhen<T>>>, RouterError> {
        self.inner.resolve(matching)
    }
}
