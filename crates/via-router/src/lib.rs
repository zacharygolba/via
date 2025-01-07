#![forbid(unsafe_code)]

mod path;
mod routes;
mod visitor;

pub use path::{Param, Span};
pub use visitor::{Found, VisitError};

use path::Pattern;
use routes::{Node, RouteEntry};

pub struct Router<T> {
    /// A collection of nodes that represent the path segments of a route.
    nodes: Vec<Node>,

    /// A vector of routes associated with the nodes in the route tree.
    routes: Vec<T>,
}

pub struct Endpoint<'a, T> {
    router: &'a mut Router<T>,
    key: usize,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        let mut nodes = Vec::new();
        let routes = Vec::new();

        nodes.push(Node::new(Pattern::Root));

        Self { nodes, routes }
    }

    /// Returns a reference to the route associated with the given key.
    ///
    pub fn get(&self, key: usize) -> Option<&T> {
        self.routes.get(key)
    }

    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = path::patterns(path);
        let key = insert(self, &mut segments, 0);

        Endpoint { router: self, key }
    }

    pub fn visit(&self, path: &str) -> Vec<Result<Found, VisitError>> {
        let mut segments = vec![];

        path::split(&mut segments, path);
        visitor::visit(path, &self.nodes, &segments)
    }
}

impl<T> Router<T> {
    /// Returns a mutable representation of a single node in the route store.
    fn entry(&mut self, key: usize) -> RouteEntry<T> {
        RouteEntry::new(self, key)
    }

    /// Pushes a new node into the store and returns the key of the newly
    /// inserted node.
    fn push(&mut self, node: Node) -> usize {
        let key = self.nodes.len();

        self.nodes.push(node);
        key
    }

    /// Returns a shared reference to the node at the given `key`.
    fn node(&self, key: usize) -> &Node {
        &self.nodes[key]
    }

    /// Returns a mutable reference to the node at the given `key`.
    fn node_mut(&mut self, key: usize) -> &mut Node {
        &mut self.nodes[key]
    }

    /// Returns a mutable reference to the route at the given `key`.
    ///
    fn get_mut(&mut self, key: usize) -> &mut T {
        &mut self.routes[key]
    }

    /// Pushes a new route into the store and returns the index of the newly
    /// inserted route.
    fn push_route(&mut self, route: T) -> usize {
        let index = self.routes.len();
        self.routes.push(route);
        index
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
        let key = insert(self.router, &mut segments, self.key);

        Endpoint {
            router: self.router,
            key,
        }
    }

    pub fn param(&self) -> Option<&Param> {
        self.router.node(self.key).param()
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
        let route_index = self.router.entry(self.key).get_or_insert_route_with(f);

        // Return a mutable reference to the route associated with this `Endpoint`.
        self.router.get_mut(route_index)
    }
}

fn insert<T, I>(router: &mut Router<T>, segments: &mut I, into_index: usize) -> usize
where
    I: Iterator<Item = Pattern>,
{
    // If the current node is a catch-all, we can skip the rest of the segments.
    // In the future we may want to panic if the caller tries to insert a node
    // into a catch-all node rather than silently ignoring the rest of the
    // segments.
    if let Pattern::Wildcard(_) = router.node(into_index).pattern {
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
    for next_index in router.node(into_index).entries() {
        if pattern == router.node(*next_index).pattern {
            return insert(router, segments, *next_index);
        }
    }

    let next_index = router.entry(into_index).push(Node::new(pattern));

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(router, segments, next_index)
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
                let found = matches[0].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(found.at, None);
                assert!(found.is_leaf);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(found.at, None);
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
                let found = matches[0].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(found.at, None);
                assert!(!found.is_leaf);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

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
                let found = matches[0].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));

                assert_eq!(route, None);
                assert_eq!(found.at, None);
                assert_eq!(found.param, None);
                assert!(!found.is_leaf);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }

            {
                // /echo/hello/world
                //  ^^^^ as Pattern::Static("echo")
                let found = matches[2].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "echo");
                assert!(!found.is_leaf);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[3].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

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
                let found = matches[0].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));

                assert_eq!(route, None);
                assert_eq!(found.at, None);
                assert_eq!(found.param, None);
                assert!(!found.is_leaf);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }

            {
                // /articles/100
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = matches[2].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "articles");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let found = matches[3].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

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
                let found = matches[0].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));

                assert_eq!(route, None);
                assert_eq!(found.at, None);
                assert_eq!(found.param, None);
                assert!(!found.is_leaf);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("path")));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.is_leaf);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = matches[2].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, None);
                assert_eq!(found.param, None);
                assert_eq!(segment, "articles");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let found = matches[3].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, Some(Param::new("id")));
                assert_eq!(segment, "100");
                assert!(!found.is_leaf);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let found = matches[4].as_ref().unwrap();
                let route = found.route.and_then(|key| router.get(key));
                let segment = {
                    let range = found.at.as_ref().unwrap();
                    &path[range.start()..range.end()]
                };

                assert_eq!(route, Some(&()));
                assert_eq!(found.param, None);
                assert_eq!(segment, "comments");
                // Should be considered exact because it is the last path segment.
                assert!(found.is_leaf);
            }
        }
    }
}
