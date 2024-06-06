mod iter;
mod node;

use crate::iter::Segments;
use crate::node::*;

pub use iter::{Match, Visit};
pub use node::Pattern;

#[derive(Clone, Debug)]
pub struct Router<T> {
    root: Node<T>,
}

#[derive(Debug)]
pub struct Endpoint<'a, T> {
    node: &'a mut Node<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Router {
            root: Node::new(Pattern::Root),
        }
    }

    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = Segments::new(path).patterns();

        Endpoint {
            node: self.root.insert(&mut segments),
        }
    }

    pub fn root(&self) -> Option<&T> {
        self.root.route.as_ref()
    }

    pub fn root_mut(&mut self) -> Option<&mut T> {
        self.root.route.as_mut()
    }

    pub fn visit<'a, 'b>(&'a self, path: &'b str) -> Visit<'a, 'b, T> {
        Visit::new(&self.root, path)
    }
}

impl<'a, T> Endpoint<'a, T> {
    pub fn at(&'a mut self, path: &'static str) -> Self {
        let mut segments = Segments::new(path).patterns();

        Endpoint {
            node: self.node.insert(&mut segments),
        }
    }

    pub fn param(&'a self) -> Option<&'static str> {
        match self.node.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }

    pub fn route_mut(&mut self) -> &mut Option<T> {
        &mut self.node.route
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::PartialEq;

    type Router = super::Router<Path>;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    struct Path(Option<&'static str>);

    impl PartialEq<str> for Path {
        fn eq(&self, other: &str) -> bool {
            if let Some(value) = self.0 {
                value == other
            } else {
                false
            }
        }
    }

    macro_rules! at {
        ($target:expr, $path:expr) => {
            let _ = $target.at($path).route_mut().insert(Path(Some($path)));
        };
    }

    macro_rules! visit {
        ($target:expr, $path:expr) => {
            $target.visit($path).last().unwrap().route().unwrap()
        };
    }

    #[test]
    fn ordering() {
        let mut router = Router::new();

        at!(router, "/*path");
        at!(router, "/echo/*path");
        at!(router, "/articles/:id");
        at!(router, "/articles/:id/comments");

        assert!(visit!(router, "/not/a/path") == "/*path");
        assert!(visit!(router, "/articles/100") == "/articles/:id");
        assert!(visit!(router, "/echo/hello/world") == "/echo/*path");
        assert!(visit!(router, "/articles/100/comments") == "/articles/:id/comments");
    }
}
