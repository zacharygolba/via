mod iter;
mod node;
mod verb;

use std::ops::{Deref, DerefMut};

use crate::iter::Segments;
use crate::node::*;

pub use iter::{Match, Visit};
pub use node::Pattern;
pub use verb::Verb;

#[derive(Clone, Debug, Default)]
pub struct Router<T> {
    root: Node<T>,
}

#[derive(Debug)]
pub struct Location<'a, T> {
    node: &'a mut Node<T>,
}

impl<T: Default> Router<T> {
    pub fn new() -> Router<T> {
        Default::default()
    }

    pub fn at(&mut self, path: &'static str) -> Location<T> {
        let mut segments = Segments::new(path).patterns();

        Location {
            node: self.root.insert(&mut segments),
        }
    }

    pub fn visit<'a, 'b>(&'a self, path: &'b str) -> Visit<'a, 'b, T> {
        Visit::new(&self.root, path)
    }
}

impl<T: Default> Deref for Router<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.root.route
    }
}

impl<T: Default> DerefMut for Router<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.root.route
    }
}

impl<'a, T: Default> Location<'a, T> {
    pub fn at(&mut self, path: &'static str) -> Location<T> {
        let mut segments = Segments::new(path).patterns();

        Location {
            node: self.node.insert(&mut segments),
        }
    }

    pub fn param(&self) -> Option<&'static str> {
        match self.node.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }
}

impl<'a, T: Default> Deref for Location<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.node.route
    }
}

impl<'a, T: Default> DerefMut for Location<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
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
            *$target.at($path) = Path(Some($path));
        };
    }

    macro_rules! visit {
        ($target:expr, $path:expr) => {
            $target.visit($path).last().unwrap().route()
        };
    }

    #[test]
    fn ordering() {
        let mut router = Router::default();

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
