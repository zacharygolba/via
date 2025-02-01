use crate::path::{self, Param, Pattern};
use crate::search::{search, Found, Match};

#[cfg(feature = "lru-cache")]
use crate::cache::Cache;

/// A node in the route tree that represents a single path segment.
pub struct Node<T> {
    /// The index of the route in the route store associated with the node.
    pub route: Option<T>,

    /// The pattern used to match the node against a path segment.
    pub pattern: Pattern,

    /// The indices of the nodes that are reachable from the current node.
    pub children: Option<Vec<usize>>,
}

pub struct Route<'a, T> {
    /// A mutable reference to the router.
    router: &'a mut Router<T>,

    /// The key of the node associated with this route.
    key: usize,
}

pub struct Router<T> {
    /// A simple LRU-cache.
    #[cfg(feature = "lru-cache")]
    cache: Cache,

    /// A collection of nodes that represent the path segments of a route.
    nodes: Vec<Node<T>>,
}

fn insert<T, I>(router: &mut Router<T>, segments: &mut I, parent_key: usize) -> usize
where
    I: Iterator<Item = Pattern>,
{
    // If the current node is a catch-all, we can skip the rest of the segments.
    // In the future we may want to panic if the caller tries to insert a node
    // into a catch-all node rather than silently ignoring the rest of the
    // segments.
    if let Pattern::Wildcard(_) = &router.nodes[parent_key].pattern {
        return parent_key;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return parent_key,
    };

    // Check if the pattern already exists in the node at `current_key`. If it
    // does, we can continue to the next segment.
    if let Some(children) = &router.nodes[parent_key].children {
        for key in children.iter().copied() {
            if pattern == router.nodes[key].pattern {
                return insert(router, segments, key);
            }
        }
    }

    let next_key = router.nodes.len();
    router.nodes.push(Node::new(pattern));

    let parent = &mut router.nodes[parent_key];
    parent.children.get_or_insert_default().push(next_key);

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(router, segments, next_key)
}

impl<T> Node<T> {
    fn new(pattern: Pattern) -> Self {
        Self {
            route: None,
            pattern,
            children: None,
        }
    }

    /// Returns an optional reference to the name of the dynamic parameter
    /// associated with the node. The returned value will be `None` if the
    /// node has a `Root` or `Static` pattern.
    #[inline]
    fn param(&self) -> Option<&Param> {
        if let Pattern::Dynamic(param) | Pattern::Wildcard(param) = &self.pattern {
            Some(param)
        } else {
            None
        }
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
        self.router.nodes.get(self.key)?.param()
    }

    /// Returns a mutable reference to the route associated with this `Endpoint`.
    /// If the route does not exist, the route will be set to the result of the
    /// provided closure `f`.
    pub fn get_or_insert_route_with<F>(&mut self, f: F) -> &mut T
    where
        F: FnOnce() -> T,
    {
        let node = &mut self.router.nodes[self.key];
        node.route.get_or_insert_with(f)
    }
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "lru-cache")]
            cache: Cache::new(1000),
            nodes: vec![Node::new(Pattern::Root)],
        }
    }

    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(self, &mut segments, 0);

        Route { router: self, key }
    }

    #[cfg(feature = "lru-cache")]
    pub fn visit(&self, path: &str) -> Vec<Match> {
        match self.cache.try_read(path) {
            Some(Some(matches)) => {
                if cfg!(debug_assertions) {
                    // Placeholder for tracing...
                    println!("via-router: cache hit {}", path);
                }
                matches
            }
            Some(None) | None => {
                if cfg!(debug_assertions) {
                    // Placeholder for tracing...
                    println!("via-router: cache miss {}", path);
                }

                let matches = search(&self.nodes, path);
                self.cache.try_write(path, &matches);
                matches
            }
        }
    }

    #[cfg(not(feature = "lru-cache"))]
    pub fn visit(&self, path: &str) -> Vec<Match> {
        search(&self.nodes, path)
    }

    #[inline]
    pub fn resolve(&self, matching: Match) -> Option<Found<T>> {
        let (exact, key, range) = matching.try_load()?;
        let node = self.nodes.get(key)?;

        Some(Found {
            exact,
            range,
            param: node.param(),
            route: node.route.as_ref(),
        })
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self::new()
    }
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
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched).unwrap())
                .collect();

            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let found = &matches[0];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert_eq!(found.range, None);
                assert!(found.exact);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let found = &matches[1];

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(found.range, None);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }
        }

        {
            let path = "/not/a/path";
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched).unwrap())
                .collect();

            assert_eq!(matches.len(), 2);

            {
                // /not/a/path
                // ^ as Pattern::Root
                let found = &matches[0];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert_eq!(found.range, None);
                assert!(!found.exact);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range[0]..range[1]]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }
        }

        {
            let path = "/echo/hello/world";
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched).unwrap())
                .collect();

            assert_eq!(matches.len(), 4);

            {
                // /echo/hello/world
                // ^ as Pattern::Root
                let found = &matches[0];

                assert_eq!(found.route, None);
                assert_eq!(found.range, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range[0]..range[1]]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^ as Pattern::Static("echo")
                let found = &matches[2];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[3];
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range[0]..range[1]]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, "hello/world");
                assert!(found.exact);
            }
        }

        {
            let path = "/articles/100";
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched).unwrap())
                .collect();

            assert_eq!(matches.len(), 4);

            {
                // /articles/100
                // ^ as Pattern::Root
                let found = &matches[0];

                assert_eq!(found.route, None);
                assert_eq!(found.range, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range[0]..range[1]]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = &matches[2];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let found = &matches[3];
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range[0]..range[1]]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"id".into()));
                assert_eq!(segment, "100");
                assert!(found.exact);
            }
        }

        {
            let path = "/articles/100/comments";
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched).unwrap())
                .collect();

            assert_eq!(matches.len(), 5);

            {
                // /articles/100/comments
                // ^ as Pattern::Root
                let found = &matches[0];

                assert_eq!(found.route, None);
                assert_eq!(found.range, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let found = &matches[1];
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range[0]..range[1]]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^ as Pattern::Static("articles")
                let found = &matches[2];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let found = &matches[3];
                let segment = {
                    let range = found.range.as_ref().unwrap();
                    &path[range[0]..range[1]]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"id".into()));
                assert_eq!(segment, "100");
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let found = &matches[4];

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, None);
                // Should be considered exact because it is the last path segment.
                assert!(found.exact);
            }
        }
    }
}
