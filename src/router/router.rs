use std::fmt::{self, Display, Formatter};

use super::route::{MatchWhen, Route};

#[derive(Debug)]
pub struct RouterError {
    message: String,
}

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

    pub(crate) fn routes(&self) -> &via_router::Router<Vec<MatchWhen<T>>> {
        &self.inner
    }
}

impl RouterError {
    pub(crate) fn new() -> Self {
        Self {
            message: "an error occurred when routing the request".to_owned(),
        }
    }
}

impl std::error::Error for RouterError {}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}
