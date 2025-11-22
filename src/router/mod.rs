pub mod method;
mod route;

pub(crate) use method::MethodNotAllowed;
pub use route::Route;

use std::sync::Arc;
use via_router::{Router as Tree, Traverse};

use crate::middleware::Middleware;

#[macro_export]
macro_rules! resources {
    ($module:path) => {
        (
            $crate::resources!($module as collection),
            $crate::resources!($module as member),
        )
    };
    ($module:path as collection) => {{
        use $module::{create, index};
        $crate::post(create).get(index)
    }};
    ($module:path as member) => {{
        use $module::{destroy, show, update};
        $crate::delete(destroy).patch(update).get(show)
    }};
    ($module:path as $other:ident) => {{
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
