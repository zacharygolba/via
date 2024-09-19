use std::sync::Arc;

use crate::Middleware;
use crate::{Endpoint, Router};

pub struct App<State> {
    state: Arc<State>,
    router: Router<State>,
}

/// Constructs a new `App` with the provided `state`.
pub fn new<State>(state: State) -> App<State>
where
    State: Send + Sync + 'static,
{
    App {
        state: Arc::new(state),
        router: Router::new(),
    }
}

impl<State> App<State>
where
    State: Send + Sync + 'static,
{
    pub fn at(&mut self, pattern: &'static str) -> Endpoint<State> {
        self.router.at(pattern)
    }

    pub fn include(&mut self, middleware: impl Middleware<State> + 'static) -> &mut Self {
        self.at("/").include(middleware);
        self
    }
}

impl<State> App<State>
where
    State: Send + Sync + 'static,
{
    pub(crate) fn into_parts(self) -> (Arc<State>, Router<State>) {
        (self.state, self.router)
    }
}
