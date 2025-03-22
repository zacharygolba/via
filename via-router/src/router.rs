use crate::error::Error;
use crate::path::{self, Param, Pattern, Split};

#[cfg(feature = "lru-cache")]
use crate::cache::Cache;

pub(crate) type Binding = (usize, Option<[usize; 2]>);

/// A node in the route tree that represents a single path segment.
pub struct Node<T> {
    /// The pattern used to match the node against a path segment.
    pub(crate) pattern: Pattern,

    /// The indices of the nodes that are reachable from the current node.
    children: Option<Vec<usize>>,

    /// The index of the route in the route store associated with the node.
    route: Option<T>,
}

pub struct Found<'a, T> {
    pub exact: bool,
    pub param: Option<&'a Param>,
    pub route: Option<&'a T>,
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

#[inline]
fn found(exact: bool, key: usize, range: Option<[usize; 2]>) -> Binding {
    (
        (key << 0b10) | (1 << 0b00) | (if exact { 1 } else { 0 } << 0b01),
        range,
    )
}

#[inline]
fn not_found(range: Option<[usize; 2]>) -> Binding {
    (0, range)
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
    pub fn param(&self) -> Option<&Param> {
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
    pub fn visit(&self, path: &str) -> Vec<Binding> {
        let mut attempts = 0;

        let cache = &self.cache;
        let nodes = &self.nodes;

        loop {
            return match cache.read(path) {
                // The requested resource was available in cache.
                Ok(Some((key, matches))) => {
                    cache.promote(key);
                    matches
                }

                // The requested resource is not available in cache.
                Ok(None) => {
                    let matches = lookup(nodes, path);

                    cache.write(path, &matches);
                    matches
                }

                // The cache is unavailable due to a write conflict.
                Err(_) if attempts > 0 => lookup(nodes, path),

                // The cache is unavailable due to a write conflict.
                Err(_) => {
                    attempts += 1;
                    continue;
                }
            };
        }
    }

    #[cfg(not(feature = "lru-cache"))]
    pub fn visit(&self, path: &str) -> Vec<Binding> {
        lookup(&self.nodes, path)
    }

    #[inline]
    pub fn resolve(&self, key: usize) -> Result<Found<T>, Error> {
        let node = if key & 0b01 != 0 {
            self.nodes.get(key >> 0b10).ok_or_else(Error::new)?
        } else {
            return Err(Error::new());
        };

        Ok(Found {
            exact: (key & 0b10) != 0,
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

fn lookup<T>(nodes: &[Node<T>], path: &str) -> Vec<Binding> {
    let root = match nodes.first() {
        Some(next) => next,
        None => return vec![],
    };

    let mut results = Vec::with_capacity(8);
    let mut segments = Vec::with_capacity(8);

    segments.extend(Split::new(path));
    results.push(found(segments.is_empty(), 0, None));

    if let Some(match_next) = &root.children {
        rlookup(&mut results, nodes, match_next, path, &segments, 0);
    }

    results
}

/// Recursively search for nodes that match the uri path.
fn rlookup<T>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Binding>,

    // A reference to the route store that contains the route tree.
    nodes: &[Node<T>],

    // A slice containing the indices of the nodes to match against the current
    // path segment at `index`.
    match_now: &[usize],

    // A str containing the entire url path.
    path: &str,

    // A reference to the range of each segment separated by / in `path`.
    segments: &[[usize; 2]],

    // The index of the path segment to match against `match_now` in `segments`.
    index: usize,
) {
    let next_index = index + 1;
    let match_range = segments.get(index).copied();
    let has_remaining = segments.get(next_index).is_none();

    for key in match_now.iter().cloned() {
        let node = match nodes.get(key) {
            Some(next) => next,
            None => {
                results.push(not_found(match_range));
                continue;
            }
        };

        match &node.pattern {
            Pattern::Static(name) => match match_range {
                // The node has a static pattern that matches the path segment.
                Some([start, end]) if name == &path[start..end] => {
                    results.push(found(has_remaining, key, match_range));
                }
                Some(_) => continue,
                None => break,
            },

            // The node has a dynamic pattern that can match any value.
            Pattern::Dynamic(_) => {
                results.push(found(has_remaining, key, match_range));
            }

            // The node has a wildcard pattern that can match any value
            // and consume the remainder of the uri path.
            Pattern::Wildcard(_) => {
                let match_range = match_range.map(|[start, _]| [start, path.len()]);
                results.push(found(true, key, match_range));
                continue;
            }

            // A root node cannot be an edge. If this branch is matched, it is
            // indicative of a bug in this crate.
            Pattern::Root => {
                // Placeholder for tracing...
                continue;
            }
        }

        if let Some(match_next) = &node.children {
            rlookup(results, nodes, match_next, path, segments, next_index)
        }
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
                .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
                .collect();

            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let (range, found) = &matches[0];

                assert_eq!(*range, None);
                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(found.exact);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let (range, found) = &matches[1];

                assert_eq!(*range, None);
                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"path".into()));
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }
        }

        {
            let path = "/not/a/path";
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
                .collect();

            assert_eq!(matches.len(), 2);

            {
                // /not/a/path
                // ^ as Pattern::Root
                let (range, found) = &matches[0];

                assert_eq!(*range, None);
                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (range, found) = &matches[1];
                let segment = {
                    let [start, end] = range.unwrap();
                    &path[start..end]
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
                .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
                .collect();

            assert_eq!(matches.len(), 4);

            {
                // /echo/hello/world
                // ^ as Pattern::Root
                let (range, found) = &matches[0];

                assert_eq!(*range, None);
                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (range, found) = &matches[1];
                let segment = {
                    let [start, end] = range.unwrap();
                    &path[start..end]
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
                let (_, found) = &matches[2];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (range, found) = &matches[3];
                let segment = {
                    let [start, end] = range.unwrap();
                    &path[start..end]
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
                .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
                .collect();

            assert_eq!(matches.len(), 4);

            {
                // /articles/100
                // ^ as Pattern::Root
                let (range, found) = &matches[0];
                assert_eq!(*range, None);

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (range, found) = &matches[1];
                let segment = {
                    let [start, end] = range.unwrap();
                    &path[start..end]
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
                let (_, found) = &matches[2];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let (range, found) = &matches[3];
                let segment = {
                    let [start, end] = range.unwrap();
                    &path[start..end]
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
                .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
                .collect();

            assert_eq!(matches.len(), 5);

            {
                // /articles/100/comments
                // ^ as Pattern::Root
                let (range, found) = &matches[0];

                assert_eq!(*range, None);
                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (range, found) = &matches[1];
                let segment = {
                    let [start, end] = range.unwrap();
                    &path[start..end]
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
                let (_, found) = &matches[2];

                assert_eq!(found.route, None);
                assert_eq!(found.param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let (range, found) = &matches[3];
                let segment = {
                    let [start, end] = range.unwrap();
                    &path[start..end]
                };

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, Some(&"id".into()));
                assert_eq!(segment, "100");
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let (_, found) = &matches[4];

                assert_eq!(found.route, Some(&()));
                assert_eq!(found.param, None);
                // Should be considered exact because it is the last path segment.
                assert!(found.exact);
            }
        }
    }
}
