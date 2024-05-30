mod iter;
mod node;
mod verb;

use std::ops::{Deref, DerefMut};

use crate::{iter::*, node::*};

pub use iter::{Component, Visit};
pub use node::Pattern;
pub use verb::Verb;

#[derive(Debug)]
pub struct Location<'a, T>(&'a mut Node<T>);

#[derive(Clone, Debug, Default)]
pub struct Router<T>(Node<T>);

impl<'a, T: Default> Location<'a, T> {
    pub fn at(&mut self, path: &'static str) -> Location<T> {
        let mut segments = Path::segments(path);
        Location(self.0.insert(&mut segments))
    }

    pub fn param(&self) -> Option<&'static str> {
        match self.0.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }
}

impl<'a, T: Default> Deref for Location<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.route
    }
}

impl<'a, T: Default> DerefMut for Location<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.route
    }
}

impl<T: Default> Router<T> {
    pub fn new() -> Router<T> {
        Default::default()
    }

    pub fn at(&mut self, path: &'static str) -> Location<T> {
        let mut segments = Path::segments(path);
        Location(self.0.insert(&mut segments))
    }

    pub fn visit<'a, 'b>(&'a self, path: &'b str) -> Visit<'a, 'b, T> {
        Visit::root(&self.0, path)
    }
}

impl<T: Default> Deref for Router<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.route
    }
}

impl<T: Default> DerefMut for Router<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.route
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
            $target.visit($path).last().unwrap().route
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
