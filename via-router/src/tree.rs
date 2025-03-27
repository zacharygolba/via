use smallvec::SmallVec;
use std::sync::Arc;
use std::{mem, slice};

use crate::{
    path::{self, Param, Pattern, Split},
    Error,
};

#[derive(Clone, Debug)]
pub enum MatchCond<T> {
    Partial(T),
    Exact(T),
}

#[derive(Debug)]
pub struct Binding {
    nodes: Vec<MatchCond<usize>>,
    range: Option<[usize; 2]>,
}

#[derive(Debug)]
pub struct Router<T> {
    tree: Vec<Node<T>>,
}

pub struct Route<'a, T> {
    tree: &'a mut Vec<Node<T>>,
    key: usize,
}

#[derive(Debug)]
pub struct Node<T> {
    children: Vec<usize>,
    pattern: Pattern,
    route: Option<T>,
}

fn insert<T, I>(tree: &mut Vec<Node<T>>, segments: &mut I, parent_key: usize) -> usize
where
    I: Iterator<Item = Pattern>,
{
    // If the current node is a catch-all, we can skip the rest of the segments.
    // In the future we may want to panic if the caller tries to insert a node
    // into a catch-all node rather than silently ignoring the rest of the
    // segments.
    if let Pattern::Wildcard(_) = &tree[parent_key].pattern {
        return parent_key;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return parent_key,
    };

    // Check if the pattern already exists in the node at `current_key`. If it
    // does, we can continue to the next segment.
    for key in tree[parent_key].children.iter().copied() {
        if pattern == tree[key].pattern {
            return insert(tree, segments, key);
        }
    }

    let next_key = tree.len();
    tree.push(Node::new(pattern));

    let parent = &mut tree[parent_key];
    parent.children.push(next_key);

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(tree, segments, next_key)
}

impl Binding {
    pub fn iter(&self) -> slice::Iter<MatchCond<usize>> {
        self.nodes.iter()
    }

    pub fn range(&self) -> Option<&[usize; 2]> {
        self.range.as_ref()
    }
}

impl Binding {
    fn new(range: Option<[usize; 2]>) -> Self {
        Self {
            nodes: Vec::with_capacity(3),
            range,
        }
    }

    fn push(&mut self, key: MatchCond<usize>) {
        self.nodes.push(key);
    }
}

impl<T> MatchCond<T> {
    pub fn as_either(&self) -> &T {
        match self {
            Self::Exact(exact) => exact,
            Self::Partial(partial) => partial,
        }
    }

    fn as_partial(&self) -> Option<&T> {
        if let Self::Partial(partial) = self {
            Some(partial)
        } else {
            None
        }
    }
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            tree: vec![Node::new(Pattern::Root)],
        }
    }

    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(&mut self.tree, &mut segments, 0);

        Route {
            tree: &mut self.tree,
            key,
        }
    }

    pub fn get(&self, key: usize) -> Result<(&Pattern, &Option<T>), Error> {
        match self.tree.get(key) {
            Some(node) => Ok((&node.pattern, &node.route)),
            None => Err(Error::new()),
        }
    }

    pub fn visit(&self, path: &str) -> Vec<Binding> {
        let mut segments = Split::new(path).peekable();
        let mut results = Vec::with_capacity(8);
        let mut queue = SmallVec::<[usize; 4]>::new();
        let mut next = SmallVec::<[usize; 4]>::new();
        let tree = &self.tree;

        loop {
            let mut binding = Binding::new(segments.next());
            let is_exact = segments.peek().is_none();
            let range = match &binding.range {
                Some(range) => {
                    next.clear();
                    *range
                }
                None => {
                    for key in queue.iter().copied() {
                        let pattern = tree.get(key).map(|node| &node.pattern);
                        if let Some(Pattern::Wildcard(_)) = pattern {
                            binding.push(MatchCond::Exact(key))
                        }
                    }

                    return results;
                }
            };

            for key in queue.iter().copied() {
                let node = match tree.get(key) {
                    Some(node) => node,
                    None => {
                        // Placeholder for tracing...
                        continue;
                    }
                };

                let match_cond = match &node.pattern {
                    Pattern::Dynamic(_) => {
                        if is_exact {
                            MatchCond::Exact(key)
                        } else {
                            MatchCond::Partial(key)
                        }
                    }

                    Pattern::Wildcard(_) => MatchCond::Exact(key),

                    Pattern::Static(label) => {
                        let [start, end] = range;

                        if label != &path[start..end] {
                            continue;
                        }

                        if is_exact {
                            MatchCond::Exact(key)
                        } else {
                            MatchCond::Partial(key)
                        }
                    }

                    Pattern::Root => {
                        // Placeholder for tracing...
                        continue;
                    }
                };

                binding.push(match_cond);
                next.extend_from_slice(&node.children);
            }

            results.push(binding);
            mem::swap(&mut queue, &mut next);
        }
    }
}

impl<T> Route<'_, T> {
    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(&mut self.tree, &mut segments, self.key);

        Route {
            tree: &mut self.tree,
            key,
        }
    }

    pub fn as_mut(&mut self) -> &mut Option<T> {
        &mut self.tree[self.key].route
    }
}

impl<T> Node<T> {
    fn new(pattern: Pattern) -> Self {
        Self {
            children: Vec::new(),
            pattern,
            route: None,
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
            let _ = router.at(path).as_mut().insert(());
        }

        println!("router: {:?}", router);

        // {
        //     let path = "/";
        //     let matches: Vec<_> = router.visit(path);

        //     println!("match / {:?}", matches);
        //     assert_eq!(matches.len(), 2);

        //     {
        //         // /
        //         // ^ as Pattern::Root
        //         let binding = &matches[0];

        //         assert!(binding.is_exact);

        //         assert_eq!(binding.to, None);
        //         assert_eq!(binding.param(), None);
        //         assert_eq!(binding.node.route.len(), 0);
        //     }

        //     {
        //         // /
        //         //  ^ as Pattern::CatchAll("*path")
        //         let binding = &matches[1];

        //         assert_eq!(binding.to, None);
        //         assert_eq!(binding.node.route.len(), 1);
        //         assert_eq!(binding.param(), Some((&"path".into(), None)));
        //         // Should be considered exact because of the catch-all pattern.
        //         assert!(binding.is_exact);
        //     }
        // }

        // {
        //     let path = "/not/a/path";
        //     let matches: Vec<_> = router
        //         .visit(path)
        //         .into_iter()
        //         .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
        //         .collect();

        //     assert_eq!(matches.len(), 2);

        //     {
        //         // /not/a/path
        //         // ^ as Pattern::Root
        //         let (range, found) = &matches[0];

        //         assert_eq!(*range, None);
        //         assert_eq!(found.route, None);
        //         assert_eq!(found.param, None);
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /not/a/path
        //         //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
        //         let (range, found) = &matches[1];
        //         let segment = {
        //             let [start, end] = range.unwrap();
        //             &path[start..end]
        //         };

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, Some(&"path".into()));
        //         assert_eq!(segment, &path[1..]);
        //         // Should be considered exact because of the catch-all pattern.
        //         assert!(found.exact);
        //     }
        // }

        // {
        //     let path = "/echo/hello/world";
        //     let matches: Vec<_> = router
        //         .visit(path)
        //         .into_iter()
        //         .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
        //         .collect();

        //     assert_eq!(matches.len(), 4);

        //     {
        //         // /echo/hello/world
        //         // ^ as Pattern::Root
        //         let (range, found) = &matches[0];

        //         assert_eq!(*range, None);
        //         assert_eq!(found.route, None);
        //         assert_eq!(found.param, None);
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /echo/hello/world
        //         //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
        //         let (range, found) = &matches[1];
        //         let segment = {
        //             let [start, end] = range.unwrap();
        //             &path[start..end]
        //         };

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, Some(&"path".into()));
        //         assert_eq!(segment, &path[1..]);
        //         // Should be considered exact because of the catch-all pattern.
        //         assert!(found.exact);
        //     }

        //     {
        //         // /echo/hello/world
        //         //  ^^^^ as Pattern::Static("echo")
        //         let (_, found) = &matches[2];

        //         assert_eq!(found.route, None);
        //         assert_eq!(found.param, None);
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /echo/hello/world
        //         //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
        //         let (range, found) = &matches[3];
        //         let segment = {
        //             let [start, end] = range.unwrap();
        //             &path[start..end]
        //         };

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, Some(&"path".into()));
        //         assert_eq!(segment, "hello/world");
        //         assert!(found.exact);
        //     }
        // }

        // {
        //     let path = "/articles/100";
        //     let matches: Vec<_> = router
        //         .visit(path)
        //         .into_iter()
        //         .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
        //         .collect();

        //     assert_eq!(matches.len(), 4);

        //     {
        //         // /articles/100
        //         // ^ as Pattern::Root
        //         let (range, found) = &matches[0];
        //         assert_eq!(*range, None);

        //         assert_eq!(found.route, None);
        //         assert_eq!(found.param, None);
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /articles/100
        //         //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
        //         let (range, found) = &matches[1];
        //         let segment = {
        //             let [start, end] = range.unwrap();
        //             &path[start..end]
        //         };

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, Some(&"path".into()));
        //         assert_eq!(segment, &path[1..]);
        //         // Should be considered exact because of the catch-all pattern.
        //         assert!(found.exact);
        //     }

        //     {
        //         // /articles/100
        //         //  ^^^^^^^^ as Pattern::Static("articles")
        //         let (_, found) = &matches[2];

        //         assert_eq!(found.route, None);
        //         assert_eq!(found.param, None);
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /articles/100
        //         //           ^^^ as Pattern::Dynamic(":id")
        //         let (range, found) = &matches[3];
        //         let segment = {
        //             let [start, end] = range.unwrap();
        //             &path[start..end]
        //         };

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, Some(&"id".into()));
        //         assert_eq!(segment, "100");
        //         assert!(found.exact);
        //     }
        // }

        // {
        //     let path = "/articles/100/comments";
        //     let matches: Vec<_> = router
        //         .visit(path)
        //         .into_iter()
        //         .filter_map(|(key, range)| Some((range, router.resolve(key).ok()?)))
        //         .collect();

        //     assert_eq!(matches.len(), 5);

        //     {
        //         // /articles/100/comments
        //         // ^ as Pattern::Root
        //         let (range, found) = &matches[0];

        //         assert_eq!(*range, None);
        //         assert_eq!(found.route, None);
        //         assert_eq!(found.param, None);
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /articles/100/comments
        //         //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
        //         let (range, found) = &matches[1];
        //         let segment = {
        //             let [start, end] = range.unwrap();
        //             &path[start..end]
        //         };

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, Some(&"path".into()));
        //         assert_eq!(segment, &path[1..]);
        //         // Should be considered exact because of the catch-all pattern.
        //         assert!(found.exact);
        //     }

        //     {
        //         // /articles/100/comments
        //         //  ^^^^^^^^ as Pattern::Static("articles")
        //         let (_, found) = &matches[2];

        //         assert_eq!(found.route, None);
        //         assert_eq!(found.param, None);
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /articles/100/comments
        //         //           ^^^ as Pattern::Dynamic(":id")
        //         let (range, found) = &matches[3];
        //         let segment = {
        //             let [start, end] = range.unwrap();
        //             &path[start..end]
        //         };

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, Some(&"id".into()));
        //         assert_eq!(segment, "100");
        //         assert!(!found.exact);
        //     }

        //     {
        //         // /articles/100/comments
        //         //               ^^^^^^^^ as Pattern::Static("comments")
        //         let (_, found) = &matches[4];

        //         assert_eq!(found.route, Some(&()));
        //         assert_eq!(found.param, None);
        //         // Should be considered exact because it is the last path segment.
        //         assert!(found.exact);
        //     }
        // }
    }
}
