#![forbid(unsafe_code)]

mod path;
mod routes;
mod visitor;

pub use path::Param;
pub use visitor::{Found, Match, RouterError};

use smallvec::SmallVec;
use std::mem;

#[cfg(feature = "lru-cache")]
use std::collections::VecDeque;
#[cfg(feature = "lru-cache")]
use std::sync::RwLock;

use path::{Pattern, Split};
use routes::Node;

pub struct Route<'a, T> {
    router: &'a mut Router<T>,
    key: usize,
}

pub struct Router<T> {
    // A simple LRU-cache.
    #[cfg(feature = "lru-cache")]
    cache: RwLock<VecDeque<(Box<str>, Vec<Match>)>>,

    /// A collection of nodes that represent the path segments of a route.
    nodes: Vec<Node<T>>,
}

pub fn search<T>(nodes: &[Node<T>], path: &str) -> Vec<Match> {
    let mut results = Vec::with_capacity(8);
    let mut match_now = SmallVec::<[&[usize]; 1]>::new();
    let mut match_next = SmallVec::<[&[usize]; 1]>::new();
    let mut path_segments = Split::new(path).peekable();

    match nodes.first() {
        Some(root) => {
            results.push(Match::found(path_segments.peek().is_none(), 0, None));
            if let Some(next) = &root.children {
                match_now.push(next);
            }
        }
        None => {
            results.push(Match::not_found());
            return results;
        }
    }

    loop {
        let path_segment = path_segments.next().map(|range| {
            let [from, to] = range;
            path.get(from..to).map(|value| (range, value))
        });

        let has_remaining = path_segments.peek().is_none();

        let mut match_count = 0;

        for children in &match_now {
            for key in children.iter() {
                let node = match nodes.get(*key) {
                    Some(child) => child,
                    None => {
                        results.push(Match::not_found());
                        continue;
                    }
                };

                let matching = match (&node.pattern, path_segment) {
                    // The node has a dynamic pattern that can match any value.
                    (Pattern::Dynamic(_), Some(Some((range, _)))) => {
                        Match::found(has_remaining, *key, Some(range))
                    }

                    // The node has a static pattern that matches the path segment.
                    (Pattern::Static(name), Some(Some((_, value)))) if name == value => {
                        Match::found(has_remaining, *key, None)
                    }

                    // The node has a wildcard pattern that can match any value
                    // and consume the remainder of the uri path.
                    (Pattern::Wildcard(_), option @ (Some(Some(_)) | None)) => {
                        let range = option.and_then(|get| get.map(|([i, _], _)| [i, path.len()]));
                        Match::found(true, *key, range)
                    }

                    // A root node cannot be an edge. This branch is unreachable.
                    (Pattern::Root, _) => {
                        // Placeholder for tracing...
                        continue;
                    }

                    // The range for the current path segment is out of bounds.
                    (_, Some(None)) => {
                        // Placeholder for tracing...
                        continue;
                    }

                    // The node didn't match the path segment.
                    _ => {
                        continue;
                    }
                };

                match_count += 1;
                results.push(matching);

                if !has_remaining {
                    if let Some(next) = &node.children {
                        match_next.push(next);
                    }
                }
            }
        }

        mem::swap(&mut match_now, &mut match_next);
        match_next.clear();

        if match_count == 0 {
            return results;
        }
    }
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "lru-cache")]
            cache: RwLock::new(VecDeque::new()),
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
        if let Some(cached) = self.try_read_from_cache(path) {
            cached
        } else {
            let matches = search(&self.nodes, path);
            self.try_write_to_cache(path, &matches);
            matches
        }
    }

    #[cfg(not(feature = "lru-cache"))]
    pub fn visit(&self, path: &str) -> Vec<Match> {
        search(&self.nodes, path)
    }

    #[inline]
    pub fn resolve(&self, matching: Match) -> Result<Found<T>, RouterError> {
        let (exact, key, range) = matching.try_load()?;
        let node = self.nodes.get(key).ok_or(RouterError)?;

        Ok(Found {
            exact,
            range,
            param: node.param(),
            route: node.route.as_ref(),
        })
    }
}

impl<T> Router<T> {
    /// Returns a shared reference to the node at the given `key`.
    fn get(&self, key: usize) -> &Node<T> {
        &self.nodes[key]
    }

    /// Returns a mutable reference to the node at the given `key`.
    fn get_mut(&mut self, key: usize) -> &mut Node<T> {
        &mut self.nodes[key]
    }

    /// Pushes a new node into the store and returns the key of the newly
    /// inserted node.
    fn push(&mut self, node: Node<T>) -> usize {
        let key = self.nodes.len();
        self.nodes.push(node);
        key
    }

    #[cfg(feature = "lru-cache")]
    fn try_write_to_cache(&self, path: &str, matches: &Vec<Match>) {
        if let Ok(mut guard) = self.cache.try_write() {
            if guard.len() == 1000 {
                guard.pop_back();
            }

            guard.push_front((path.into(), matches.to_vec()));
        }
    }

    #[cfg(feature = "lru-cache")]
    fn try_read_from_cache(&self, path: &str) -> Option<Vec<Match>> {
        let lock = &self.cache;
        let cached = {
            let guard = match lock.try_read() {
                Ok(guard) => guard,
                Err(_) => return None,
            };

            guard.iter().enumerate().find_map(|(index, (key, cached))| {
                if **key == *path {
                    Some((index, cached.to_vec()))
                } else {
                    None
                }
            })
        };

        if let Some((index, matches)) = cached {
            if cfg!(debug_assertions) {
                println!("via-router: cache hit");
            }

            if index > 500 {
                if let Ok(mut guard) = lock.try_write() {
                    guard.swap_remove_front(index);
                }
            }

            Some(matches)
        } else {
            if cfg!(debug_assertions) {
                println!("via-router: cache miss");
            }

            None
        }
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
        self.router.get(self.key).param()
    }

    /// Returns a mutable reference to the route associated with this `Endpoint`.
    /// If the route does not exist, the route will be set to the result of the
    /// provided closure `f`.
    pub fn get_or_insert_route_with<F>(&mut self, f: F) -> &mut T
    where
        F: FnOnce() -> T,
    {
        let node = self.router.get_mut(self.key);
        node.route.get_or_insert_with(f)
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
    if let Pattern::Wildcard(_) = router.get(parent_key).pattern {
        return parent_key;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return parent_key,
    };

    // Check if the pattern already exists in the node at `current_key`. If it
    // does, we can continue to the next segment.
    let existing = router.get(parent_key).children().find(|key| {
        let child = router.get(*key);
        child.pattern == pattern
    });

    if let Some(next_key) = existing {
        return insert(router, segments, next_key);
    }

    let next_key = router.push(Node::new(pattern));

    router.get_mut(parent_key).push(next_key);

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
            let matches = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

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
            let matches = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

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
            let matches = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

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
            let matches = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

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
            let matches = router
                .visit(path)
                .into_iter()
                .map(|matched| router.resolve(matched))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

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
