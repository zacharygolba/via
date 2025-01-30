#![forbid(unsafe_code)]

mod path;
mod routes;
mod visitor;

pub use path::Param;
pub use visitor::{Found, VisitError};

use smallvec::SmallVec;

#[cfg(feature = "lru-cache")]
use std::collections::VecDeque;
#[cfg(feature = "lru-cache")]
use std::sync::RwLock;

use path::{Pattern, Split};
use routes::Node;
use visitor::Match;

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
    let mut results = Vec::new();
    let mut segments = Split::new(path).peekable();
    let mut match_asap = SmallVec::<[&Node<T>; 2]>::new();
    let mut match_next = SmallVec::<[&Node<T>; 2]>::new();

    if let Some(root) = nodes.first() {
        results.push(Match::new(segments.peek().is_none(), 0, None));
        match_asap.push(root);
    } else {
        results.push(Match::default());
        return results;
    }

    while let Some(range) = segments.next() {
        let segment = &path[range[0]..range[1]];
        let is_last = segments.peek().is_none();

        for key in match_asap.iter().flat_map(|node| node.children()) {
            let child = match nodes.get(key) {
                Some(node) => node,
                None => {
                    results.push(Match::default());
                    continue;
                }
            };

            let (exact, range) = match &child.pattern {
                Pattern::Static(value) if value == segment => (is_last, None),
                Pattern::Static(_) | Pattern::Root => continue,
                Pattern::Wildcard(_) => (true, Some([range[0], path.len()])),
                Pattern::Dynamic(_) => (is_last, Some(range)),
            };

            results.push(Match::new(exact, key, range));
            match_next.push(child);
        }

        match_asap = match_next;
        match_next = SmallVec::new();
    }

    for key in match_asap.iter().flat_map(|node| node.children()) {
        match nodes.get(key) {
            Some(node) if matches!(&node.pattern, Pattern::Wildcard(_)) => {
                results.push(Match::new(true, key, None));
            }
            None => {
                results.push(Match::default());
            }
            Some(_) => {}
        }
    }

    results
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

    pub fn resolve(&self, matching: Match) -> Result<Found<T>, VisitError> {
        let (exact, key, range) = matching.try_load()?;
        let node = self.nodes.get(key).ok_or(VisitError::NodeNotFound)?;

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
            if guard.len() == 100 {
                guard.pop_back();
            }

            guard.push_front((path.into(), matches.clone()));
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
                    Some((index, cached.clone()))
                } else {
                    None
                }
            })
        };

        if let Some((index, matches)) = cached {
            if cfg!(debug_assertions) {
                println!("cache hit");
            }

            if index > 50 {
                if let Ok(mut guard) = lock.try_write() {
                    guard.swap_remove_front(index);
                }
            }

            Some(matches)
        } else {
            if cfg!(debug_assertions) {
                println!("cache miss");
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
