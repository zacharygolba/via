#![forbid(unsafe_code)]

mod iter;
mod path;
mod routes;
mod stack_vec;
mod visitor;

use stack_vec::StackVec;

pub use crate::iter::{Matched, Matches};

use crate::path::Pattern;
use crate::routes::{Node, RouteStore};
use crate::visitor::Visitor;

pub struct Router<T> {
    store: RouteStore<T>,
}

pub struct Endpoint<'a, T> {
    key: usize,
    store: &'a mut RouteStore<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        let mut store = RouteStore::new();

        store.push(Node::new(Pattern::Root));
        Self { store }
    }

    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = path::patterns(path);

        Endpoint {
            key: insert(&mut self.store, &mut segments, 0),
            store: &mut self.store,
        }
    }

    /// Shrinks the capacity of the router as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.store.shrink_to_fit();
    }

    pub fn visit<'a>(&'a self, path: &str) -> Matches<'a, T> {
        let mut results = StackVec::new();
        let segments = path::segments(path);
        let store = &self.store;

        Visitor::new(path, &segments, store).visit(&mut results);
        Matches::new(store, results.into_iter())
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> Endpoint<'a, T> {
    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = path::patterns(path);

        Endpoint {
            key: insert(self.store, &mut segments, self.key),
            store: self.store,
        }
    }

    pub fn param(&self) -> Option<&'static str> {
        self.store.get(self.key).param()
    }

    /// Returns a mutable reference to the route associated with this `Endpoint`.
    /// If the route does not exist, the route will be set to the result of the
    /// provided closure `f`.
    pub fn get_or_insert_route_with<F>(&mut self, f: F) -> &mut T
    where
        F: FnOnce() -> T,
    {
        // Get the index of the route associated with the current node or insert
        // a new route by calling the provided closure `f` if it does not exist.
        let route_index = self.store.entry(self.key).get_or_insert_route_with(f);

        // Return a mutable reference to the route associated with this `Endpoint`.
        self.store.route_mut(route_index)
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
    if let Pattern::CatchAll(_) = routes.get(into_index).pattern {
        for _ in segments {}
        return into_index;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return into_index,
    };

    // Check if the pattern already exists in the node at `current_key`. If it does,
    // we can continue to the next segment.
    for next_index in routes.get(into_index).entries() {
        if pattern == routes.get(*next_index).pattern {
            return insert(routes, segments, *next_index);
        }
    }

    let next_index = routes.entry(into_index).push(Node {
        pattern,
        entries: StackVec::new(),
        route: None,
    });

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(routes, segments, next_index)
}

#[cfg(test)]
mod tests {
    use super::Router;

    const PATHS: [&str; 4] = [
        "/*path",
        "/echo/*path",
        "/articles/:id",
        "/articles/:id/comments",
    ];

    #[test]
    fn test_router_visit() {
        let mut router = Router::new();

        for path in &PATHS {
            let _ = router.at(path).get_or_insert_route_with(|| ());
        }

        {
            let path = "/";
            let matches: Vec<_> = router.visit(path).collect();

            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let matched = &matches[0];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "");
                assert!(matched.exact);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], "");
                // Should be considered exact because of the catch-all pattern.
                assert!(matched.exact);
            }
        }

        {
            let path = "/not/a/path";
            let matches: Vec<_> = router.visit(path).collect();

            assert_eq!(matches.len(), 2);

            {
                // /not/a/path
                // ^ as Pattern::Root
                let matched = &matches[0];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "");
                assert!(!matched.exact);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(matched.exact);
            }
        }

        {
            let path = "/echo/hello/world";
            let matches: Vec<_> = router.visit(path).collect();

            assert_eq!(matches.len(), 4);

            {
                // /echo/hello/world
                // ^ as Pattern::Root
                let matched = &matches[0];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "");
                assert!(!matched.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(matched.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^ as Pattern::Static("echo")
                let matched = &matches[2];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "echo");
                assert!(!matched.exact);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[3];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], "hello/world");
                assert!(matched.exact);
            }
        }

        {
            let path = "/articles/100";
            let matches: Vec<_> = router.visit(path).collect();

            assert_eq!(matches.len(), 4);

            {
                // /articles/100
                // ^ as Pattern::Root
                let matched = &matches[0];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "");
                assert!(!matched.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(matched.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^ as Pattern::Static("articles")
                let matched = &matches[2];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "articles");
                assert!(!matched.exact);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let matched = &matches[3];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], "100");
                assert!(matched.exact);
            }
        }

        {
            let path = "/articles/100/comments";
            let matches: Vec<_> = router.visit(path).collect();

            assert_eq!(matches.len(), 5);

            {
                // /articles/100/comments
                // ^ as Pattern::Root
                let matched = &matches[0];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "");
                assert!(!matched.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(matched.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^ as Pattern::Static("articles")
                let matched = &matches[2];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, None);
                // assert_eq!(&path[start..end], "articles");
                assert!(!matched.exact);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let matched = &matches[3];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], "100");
                assert!(!matched.exact);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let matched = &matches[4];
                // let (start, end) = matched.range;

                assert_eq!(matched.route, Some(&()));
                // assert_eq!(&path[start..end], "comments");
                // Should be considered exact because it is the last path segment.
                assert!(matched.exact);
            }
        }
    }
}
