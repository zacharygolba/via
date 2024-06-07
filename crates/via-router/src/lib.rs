mod iter;
mod node;

use slab::Slab;

use crate::iter::Segments;
use crate::node::*;

pub use iter::{Match, Visit};
pub use node::Pattern;

pub(crate) type Store<T> = Slab<Node<T>>;

#[derive(Clone, Debug)]
pub struct Router<T> {
    root: usize,
    store: Store<T>,
}

#[derive(Debug)]
pub struct Endpoint<'a, T> {
    key: usize,
    store: &'a mut Store<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        let mut store = Slab::with_capacity(512);
        let root = store.insert(Node::new(Pattern::Root));

        Router { root, store }
    }

    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = Segments::new(path).patterns();

        Endpoint {
            key: insert(self.root, &mut self.store, &mut segments),
            store: &mut self.store,
        }
    }

    pub fn route(&self) -> Option<&T> {
        self.store[self.root].route.as_ref()
    }

    pub fn route_mut(&mut self) -> &mut Option<T> {
        &mut self.store[self.root].route
    }

    pub fn visit<'a, 'b>(&'a self, path: &'b str) -> Visit<'a, 'b, T> {
        Visit::new(&self.store, &self.store[self.root], path)
    }
}

impl<'a, T> Endpoint<'a, T> {
    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = Segments::new(path).patterns();

        Endpoint {
            key: insert(self.key, &mut self.store, &mut segments),
            store: self.store,
        }
    }

    pub fn param(&'a self) -> Option<&'static str> {
        let node = &self.store[self.key];

        match node.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }

    pub fn route_mut(&mut self) -> &mut Option<T> {
        &mut self.store[self.key].route
    }
}

fn insert<T, I>(key: usize, store: &mut Store<T>, segments: &mut I) -> usize
where
    I: Iterator<Item = Pattern>,
{
    if let Pattern::CatchAll(_) = store[key].pattern {
        while let Some(_) = segments.next() {}
        return key;
    }

    let pattern = match segments.next() {
        Some(value) => value,
        None => return key,
    };

    if let Some(entries) = store[key].entries.as_ref() {
        for key in entries {
            if pattern == store[*key].pattern {
                return insert(*key, store, segments);
            }
        }
    }

    let next_key = store.insert(Node::new(pattern));

    store[key]
        .entries
        .get_or_insert_with(|| Vec::with_capacity(4))
        .push(next_key);

    insert(next_key, store, segments)
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
