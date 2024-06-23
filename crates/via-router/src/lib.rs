#![forbid(unsafe_code)]

mod path;
mod routes;
mod visitor;

use crate::{
    path::{Pattern, SplitPath},
    routes::{Node, RouteStore},
    visitor::Visitor,
};

pub use crate::visitor::Match;

#[derive(Debug)]
pub struct Router<T> {
    routes: RouteStore<T>,
}

#[derive(Debug)]
pub struct Endpoint<'a, T> {
    index: usize,
    routes: &'a mut RouteStore<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        let mut routes = RouteStore::new();

        routes.insert(Node::new(Pattern::Root));
        Self { routes }
    }

    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = SplitPath::new(path).into_patterns();

        Endpoint {
            index: insert(&mut self.routes, &mut segments, 0),
            routes: &mut self.routes,
        }
    }

    pub fn visit(&self, path: &str) -> Vec<Match<T>> {
        let visitor = Visitor::new(&self.routes, path);
        let root = &self.routes[0];

        visitor.visit(root)
    }
}

impl<'a, T> Endpoint<'a, T> {
    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = SplitPath::new(path).into_patterns();

        Endpoint {
            index: insert(self.routes, &mut segments, self.index),
            routes: self.routes,
        }
    }

    pub fn param(&self) -> Option<&'static str> {
        let node = &self.routes[self.index];

        match node.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }

    pub fn route_mut(&mut self) -> &mut Option<T> {
        &mut self.routes[self.index].route
    }
}

fn insert<T, I>(routes: &mut RouteStore<T>, segments: &mut I, into_index: usize) -> usize
where
    I: Iterator<Item = Pattern>,
{
    // If the current node is a catch-all, we can skip the rest of the segments.
    // In the future we may want to panic if the caller tries to insert a node
    // into a catch-all node rather than silently ignoring the rest of the
    // segments.
    if let Pattern::CatchAll(_) = routes[into_index].pattern {
        while let Some(_) = segments.next() {}
        return into_index;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return into_index,
    };

    // Check if the pattern already exists in the node at `current_key`. If it does,
    // we can continue to the next segment.
    for next_index in routes[into_index].entries() {
        if pattern == routes[*next_index].pattern {
            return insert(routes, segments, *next_index);
        }
    }

    let next_index = routes.entry(into_index).insert(Node::new(pattern));

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(routes, segments, next_index)
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
