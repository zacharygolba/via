use crate::path::{self, Param, Pattern, Split};

#[cfg(feature = "lru-cache")]
use crate::cache::Cache;

/// A matched node in the route tree.
///
/// Contains a reference to the route associated with the node and additional
/// metadata about the match.
///
#[derive(Clone, Debug)]
pub struct Match {
    /// True if there were no more segments to match against the children of
    /// the matched node. Otherwise, false.
    ///
    pub exact: bool,

    /// The start and end offset of the parameter that matched the path
    /// segment.
    ///
    pub range: Option<[usize; 2]>,

    /// The key of the node that matched the path segment.
    ///
    key: usize,
}

/// A node in the route tree that represents a single path segment.
pub struct Node<T> {
    /// The index of the route in the route store associated with the node.
    route: Option<T>,

    /// The pattern used to match the node against a path segment.
    pattern: Pattern,

    /// The indices of the nodes that are reachable from the current node.
    children: Option<Vec<usize>>,
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

impl Match {
    #[inline]
    fn new(exact: bool, key: usize, range: Option<[usize; 2]>) -> Self {
        Self { exact, range, key }
    }
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

    #[inline]
    fn route(&self) -> Option<&T> {
        self.route.as_ref()
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
    pub fn visit(&self, path: &str) -> Vec<Option<Match>> {
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
    pub fn visit(&self, path: &str) -> Vec<Option<Match>> {
        lookup(&self.nodes, path)
    }

    #[inline]
    pub fn resolve(&self, matching: &Match) -> (Option<&Param>, Option<&T>) {
        if let Some(node) = self.nodes.get(matching.key) {
            (node.param(), node.route())
        } else {
            (None, None)
        }
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

fn lookup<T>(nodes: &[Node<T>], path: &str) -> Vec<Option<Match>> {
    let root = match nodes.first() {
        Some(next) => next,
        None => return vec![None],
    };

    let mut results = Vec::with_capacity(8);
    let mut segments = Vec::with_capacity(8);

    segments.extend(Split::new(path));
    results.push(Some(Match::new(segments.is_empty(), 0, None)));

    if let Some(match_next) = &root.children {
        rlookup(&mut results, nodes, match_next, path, &segments, 0);
    }

    results
}

/// Recursively search for nodes that match the uri path.
fn rlookup<T>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Option<Match>>,

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
                results.push(None);
                continue;
            }
        };

        match &node.pattern {
            Pattern::Static(name) => match match_range {
                // The node has a static pattern that matches the path segment.
                Some([start, end]) if name == &path[start..end] => {
                    results.push(Some(Match::new(has_remaining, key, match_range)));
                }
                Some(_) => continue,
                None => break,
            },

            // The node has a dynamic pattern that can match any value.
            Pattern::Dynamic(_) => {
                results.push(Some(Match::new(has_remaining, key, match_range)));
            }

            // The node has a wildcard pattern that can match any value
            // and consume the remainder of the uri path.
            Pattern::Wildcard(_) => {
                let match_range = match_range.map(|[start, _]| [start, path.len()]);
                results.push(Some(Match::new(true, key, match_range)));
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
                .map(|result| {
                    let matched = result.unwrap();
                    let (param, route) = router.resolve(&matched);
                    (matched, param, route)
                })
                .collect();

            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let (found, param, route) = &matches[0];

                assert_eq!(*route, None);
                assert_eq!(*param, None);
                assert_eq!(found.range, None);
                assert!(found.exact);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let (found, param, route) = &matches[1];

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"path".into()));
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
                .map(|result| {
                    let matched = result.unwrap();
                    let (param, route) = router.resolve(&matched);
                    (matched, param, route)
                })
                .collect();

            assert_eq!(matches.len(), 2);

            {
                // /not/a/path
                // ^ as Pattern::Root
                let (found, param, route) = &matches[0];

                assert_eq!(*route, None);
                assert_eq!(*param, None);
                assert_eq!(found.range, None);
                assert!(!found.exact);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (found, param, route) = &matches[1];
                let segment = {
                    let [start, end] = found.range.unwrap();
                    &path[start..end]
                };

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"path".into()));
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
                .map(|result| {
                    let matched = result.unwrap();
                    let (param, route) = router.resolve(&matched);
                    (matched, param, route)
                })
                .collect();

            assert_eq!(matches.len(), 4);

            {
                // /echo/hello/world
                // ^ as Pattern::Root
                let (found, param, route) = &matches[0];

                assert_eq!(*route, None);
                assert_eq!(*param, None);
                assert_eq!(found.range, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (found, param, route) = &matches[1];
                let segment = {
                    let [start, end] = found.range.unwrap();
                    &path[start..end]
                };

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /echo/hello/world
                //  ^^^^ as Pattern::Static("echo")
                let (found, param, route) = &matches[2];

                assert_eq!(*route, None);
                assert_eq!(*param, None);
                assert!(!found.exact);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (found, param, route) = &matches[3];
                let segment = {
                    let [start, end] = found.range.unwrap();
                    &path[start..end]
                };

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"path".into()));
                assert_eq!(segment, "hello/world");
                assert!(found.exact);
            }
        }

        {
            let path = "/articles/100";
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .map(|result| {
                    let matched = result.unwrap();
                    let (param, route) = router.resolve(&matched);
                    (matched, param, route)
                })
                .collect();

            assert_eq!(matches.len(), 4);

            {
                // /articles/100
                // ^ as Pattern::Root
                let (found, param, route) = &matches[0];

                assert_eq!(*route, None);
                assert_eq!(found.range, None);
                assert_eq!(*param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (found, param, route) = &matches[1];
                let segment = {
                    let [start, end] = found.range.unwrap();
                    &path[start..end]
                };

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /articles/100
                //  ^^^^^^^^ as Pattern::Static("articles")
                let (found, param, route) = &matches[2];

                assert_eq!(*route, None);
                assert_eq!(*param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let (found, param, route) = &matches[3];
                let segment = {
                    let [start, end] = found.range.unwrap();
                    &path[start..end]
                };

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"id".into()));
                assert_eq!(segment, "100");
                assert!(found.exact);
            }
        }

        {
            let path = "/articles/100/comments";
            let matches: Vec<_> = router
                .visit(path)
                .into_iter()
                .map(|result| {
                    let matched = result.unwrap();
                    let (param, route) = router.resolve(&matched);
                    (matched, param, route)
                })
                .collect();

            assert_eq!(matches.len(), 5);

            {
                // /articles/100/comments
                // ^ as Pattern::Root
                let (found, param, route) = &matches[0];

                assert_eq!(*route, None);
                assert_eq!(*param, None);
                assert_eq!(found.range, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let (found, param, route) = &matches[1];
                let segment = {
                    let [start, end] = found.range.unwrap();
                    &path[start..end]
                };

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"path".into()));
                assert_eq!(segment, &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert!(found.exact);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^ as Pattern::Static("articles")
                let (found, param, route) = &matches[2];

                assert_eq!(*route, None);
                assert_eq!(*param, None);
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let (found, param, route) = &matches[3];
                let segment = {
                    let [start, end] = found.range.unwrap();
                    &path[start..end]
                };

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, Some(&"id".into()));
                assert_eq!(segment, "100");
                assert!(!found.exact);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let (found, param, route) = &matches[4];

                assert_eq!(*route, Some(&()));
                assert_eq!(*param, None);
                // Should be considered exact because it is the last path segment.
                assert!(found.exact);
            }
        }
    }
}
