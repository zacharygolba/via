pub mod method;
mod route;

pub(crate) use method::MethodNotAllowed;
pub use route::Route;

use std::sync::Arc;
use via_router::{Router as Tree, Traverse};

use crate::middleware::Middleware;

#[macro_export]
macro_rules! resources {
    ($mod:path) => {
        (
            $crate::resources!($mod as collection),
            $crate::resources!($mod as member),
        )
    };
    ($mod:path as collection) => {{
        use $mod::{create, index};
        $crate::post(create).get(index)
    }};
    ($mod:path as member) => {{
        use $mod::{destroy, show, update};
        $crate::delete(destroy).patch(update).get(show)
    }};
    ($mod:path as $other:ident) => {{
        compile_error!(concat!(
            "incorrect rest! modifier \"",
            stringify!($other),
            "\"",
        ));
    }};
}

pub(crate) struct Router<T> {
    tree: Tree<Arc<dyn Middleware<T>>>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Default::default()
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
