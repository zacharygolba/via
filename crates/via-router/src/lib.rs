mod iter;
mod node;
mod routes;

use crate::{iter::Segments, node::Node, routes::RouteStore};

pub use crate::{
    iter::{Match, Visit},
    node::Pattern,
};

#[derive(Clone, Debug)]
pub struct Router<T> {
    root: usize,
    store: RouteStore<T>,
}

#[derive(Debug)]
pub struct Endpoint<'a, T> {
    key: usize,
    store: &'a mut RouteStore<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        let mut store = RouteStore::new();
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
        self.store[self.root].route()
    }

    pub fn route_mut(&mut self) -> &mut Option<T> {
        self.store[self.root].route_mut()
    }

    pub fn visit<'a, 'b>(&'a self, path: &'b str) -> Visit<'a, 'b, T> {
        let node = &self.store[self.root];
        Visit::new(&self.store, node, path)
    }
}

impl<'a, T> Endpoint<'a, T> {
    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = Segments::new(path).patterns();

        Endpoint {
            key: insert(self.key, self.store, &mut segments),
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

fn insert<T, I>(current_key: usize, route_store: &mut RouteStore<T>, segments: &mut I) -> usize
where
    I: Iterator<Item = Pattern>,
{
    // If the current node is a catch-all, we can skip the rest of the segments.
    // In the future we may want to panic if the caller tries to insert a node
    // into a catch-all node rather than silently ignoring the rest of the
    // segments.
    if let Pattern::CatchAll(_) = route_store[current_key].pattern {
        while let Some(_) = segments.next() {}
        return current_key;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return current_key,
    };

    // Check if the pattern already exists in the node at `current_key`. If it does,
    // we can continue to the next segment.
    for next_key in &route_store[current_key] {
        if pattern == route_store[*next_key].pattern {
            return insert(*next_key, route_store, segments);
        }
    }

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(
        route_store.entry(current_key).insert(Node::new(pattern)),
        route_store,
        segments,
    )
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
