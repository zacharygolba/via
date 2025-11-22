pub use resource::{Collection, Member};
pub use route::Route;

pub(crate) use resource::MethodNotAllowed;

mod resource;
mod route;

use std::sync::Arc;
use via_router::{Router as Tree, Traverse};

use crate::middleware::Middleware;

pub(crate) struct Router<T> {
    tree: Tree<Arc<dyn Middleware<T>>>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            tree: Default::default(),
        }
    }

    pub fn route(&mut self, path: &'static str) -> Route<'_, T> {
        Route {
            entry: self.tree.route(path),
        }
    }

    pub fn traverse<'b>(&self, path: &'b str) -> Traverse<'_, 'b, Arc<dyn Middleware<T>>> {
        self.tree.traverse(path)
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self {
            tree: Default::default(),
        }
    }
}
