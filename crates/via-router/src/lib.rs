#![forbid(unsafe_code)]

mod path;
mod routes;
mod visitor;

pub use path::Param;
pub use visitor::{Found, VisitError};

use smallvec::SmallVec;

use path::{Pattern, Segments};
use routes::{Node, RouteEntry};

pub struct Route<'a, T> {
    router: &'a mut Router<T>,
    key: usize,
}

pub struct Router<T> {
    /// A collection of nodes that represent the path segments of a route.
    nodes: Vec<Node>,

    /// A vector of routes associated with the nodes in the route tree.
    routes: Vec<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::new(Pattern::Root)],
            routes: vec![],
        }
    }

    /// Returns a reference to the route associated with the given key.
    ///
    pub fn get(&self, key: usize) -> Option<&T> {
        self.routes.get(key)
    }

    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(self, &mut segments, 0);

        Route { router: self, key }
    }

    pub fn visit(&self, path: &str) -> Vec<Result<Found, VisitError>> {
        // Get a shared reference to self.nodes.
        let nodes = &self.nodes;

        // Verify that the root node is present. If so, allocate a vec to store
        // the results that match `path`.
        let (mut results, root) = match nodes.first() {
            Some(node) => (Vec::new(), node),
            None => return vec![Err(VisitError::RootNotFound)],
        };

        // Split path into segment ranges and collect them into a vec.
        let segments = {
            let mut parts = SmallVec::new();

            path::split(&mut parts, path);
            Segments::new(path, parts)
        };

        // Get a shared reference to the segments in `path`.
        let segments_ref = &segments;

        // Append the root node as a match to the results vector.
        results.push(Ok(Found {
            exact: segments.is_empty(),
            param: None,
            range: None,
            route: root.route,
        }));

        // If there is at least 1 path segment to match against, perform a recursive
        // search for descendants of `root` that match the each segment in `segments`.
        // Otherwise, perform a shallow search for descendants of `root` with a
        // wildcard pattern.
        match (&root.children, segments_ref.first()) {
            // Perform a recursive search for descendants of `child` that match the next
            // path segment.
            (Some(children), Some(segment)) => {
                visitor::visit_node(&mut results, nodes, children, segments_ref, segment, 0);
            }

            // Perform a shallow search for descendants of `child` with a wildcard pattern.
            (Some(children), None) => {
                visitor::visit_wildcard(&mut results, nodes, children);
            }

            _ => {}
        }

        results
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

impl<T> Route<'_, T> {
    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(self.router, &mut segments, self.key);

        Route {
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

fn insert<T, I>(router: &mut Router<T>, segments: &mut I, parent_key: usize) -> usize
where
    I: Iterator<Item = Pattern>,
{
    // If the current node is a catch-all, we can skip the rest of the segments.
    // In the future we may want to panic if the caller tries to insert a node
    // into a catch-all node rather than silently ignoring the rest of the
    // segments.
    if let Pattern::Wildcard(_) = router.node(parent_key).pattern {
        return parent_key;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return parent_key,
    };

    // Check if the pattern already exists in the node at `current_key`. If it
    // does, we can continue to the next segment.
    if let Some(children) = &router.node(parent_key).children {
        let existing = children.iter().find(|key| {
            let child = router.node(**key);
            child.pattern == pattern
        });

        if let Some(next_key) = existing {
            return insert(router, segments, *next_key);
        }
    }

    let next_key = router.entry(parent_key).push(Node::new(pattern));

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(router, segments, next_key)
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
            let matches: Vec<_> = router.visit(path);

            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let found = matches[0].as_ref().unwrap();

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.param, None);
                assert_eq!(found.range, None);
                assert!(found.exact);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(found.range, None);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
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

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.param, None);
                assert_eq!(found.range, None);
                assert!(!found.exact);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range.0..range.1]
                };

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
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

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.range, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range.0..range.1]
                };

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^ as Pattern::Static("echo")
                let found = matches[2].as_ref().unwrap();

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[3].as_ref().unwrap();
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range.0..range.1]
                };

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, "hello/world");
                assert!(found.exact);
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

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.range, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range.0..range.1]
                };

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = matches[2].as_ref().unwrap();

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let found = matches[3].as_ref().unwrap();
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range.0..range.1]
                };

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"id".into()));
                assert_eq!(segment, "100");
                assert!(found.exact);
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

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.range, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = matches[1].as_ref().unwrap();
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range.0..range.1]
                };

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = matches[2].as_ref().unwrap();

                assert_eq!(found.route.and_then(|key| router.get(key)), None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let found = matches[3].as_ref().unwrap();
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range.0..range.1]
                };

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, Some(&"id".into()));
                assert_eq!(segment, "100");
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let found = matches[4].as_ref().unwrap();

                assert_eq!(found.route.and_then(|key| router.get(key)), Some(&()));
                assert_eq!(found.param, None);
                // Should be considered exact because it is the last path segment.
                assert!(found.exact);
            }
        }
    }
}
