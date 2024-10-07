#![forbid(unsafe_code)]

mod path;
mod routes;
mod stack_vec;
mod visitor;

pub use path::{Param, Span};
pub use visitor::Found;

use path::{Pattern, SplitPath};
use routes::{Node, RouteStore};
use stack_vec::StackVec;

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

    /// Returns a reference to the route associated with the given key.
    ///
    pub fn get(&self, key: usize) -> Option<&T> {
        self.store.route(key)
    }

    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = path::patterns(path);

        Endpoint {
            key: insert(&mut self.store, &mut segments, 0),
            store: &mut self.store,
        }
    }

    pub fn visit(&self, path: &str) -> Vec<Found> {
        let mut segments = StackVec::new([None, None, None, None, None]);
        let store = &self.store;

        for segment in SplitPath::new(path) {
            segments.push(segment);
        }

        visitor::visit(path, store, &segments)
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

    pub fn param(&self) -> Option<&Param> {
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
    if let Pattern::Wildcard(_) = routes.get(into_index).pattern {
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

    let next_index = routes.entry(into_index).push(Node::new(pattern));

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(routes, segments, next_index)
}

#[cfg(test)]
mod tests {
    use crate::path::Param;

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
            let matches: Vec<_> = router.visit(path);

            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let found = &matches[0];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "");
                assert!(found.is_leaf);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, "");
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }
        }

        {
            let path = "/not/a/path";
            let matches: Vec<_> = router.visit(path);

            assert_eq!(matches.len(), 2);

            {
                // /not/a/path
                // ^ as Pattern::Root
                let found = &matches[0];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "");
                assert!(!found.is_leaf);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }
        }

        {
            let path = "/echo/hello/world";
            let matches: Vec<_> = router.visit(path);

            assert_eq!(matches.len(), 4);

            {
                // /echo/hello/world
                // ^ as Pattern::Root
                let found = &matches[0];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "");
                assert!(!found.is_leaf);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }

            {
                // /echo/hello/world
                //  ^^^^ as Pattern::Static("echo")
                let found = &matches[2];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "echo");
                assert!(!found.is_leaf);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[3];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, "hello/world");
                assert!(found.is_leaf);
            }
        }

        {
            let path = "/articles/100";
            let matches: Vec<_> = router.visit(path);

            assert_eq!(matches.len(), 4);

            {
                // /articles/100
                // ^ as Pattern::Root
                let found = &matches[0];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }

            {
                // /articles/100
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = &matches[2];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "articles");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let found = &matches[3];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("id")));
                assert_eq!(segment, "100");
                assert!(found.is_leaf);
            }
        }

        {
            let path = "/articles/100/comments";
            let matches: Vec<_> = router.visit(path);

            assert_eq!(matches.len(), 5);

            {
                // /articles/100/comments
                // ^ as Pattern::Root
                let found = &matches[0];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = &matches[2];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "articles");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let found = &matches[3];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("id")));
                assert_eq!(segment, "100");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let found = &matches[4];
                let route = found.route.and_then(|key| router.get(key));
                let segment = &path[found.at.start()..found.at.end()];

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, None);
                assert_eq!(segment, "comments");
                // Should be considered exact because it is the last path segment.
                assert!(found.is_leaf);
            }
        }
    }
}
