use either::Either;
use smallvec::SmallVec;
use std::fmt::{self, Debug, Formatter};
use std::{mem, slice};

use crate::path::{self, Param, Pattern, Split};

#[derive(Clone, Debug)]
pub enum MatchCond<T> {
    Partial(T),
    Exact(T),
}

pub struct Binding<'a, T> {
    range: Option<[usize; 2]>,
    nodes: Vec<MatchCond<&'a Node<T>>>,
}

#[derive(Debug)]
pub struct Router<T> {
    tree: Vec<Node<T>>,
}

pub struct Route<'a, T> {
    tree: &'a mut Vec<Node<T>>,
    key: usize,
}

pub struct Node<T> {
    children: Either<Option<usize>, Vec<usize>>,
    pattern: Pattern,
    route: Option<Vec<T>>,
}

macro_rules! lookup {
    ($tree:expr, $key:expr) => {
        match $tree.get($key) {
            Some(node) => node,
            None => {
                // Placeholder for tracing...
                //
                // This should never happen but we def want to know
                // if it does.
                continue;
            }
        }
    };
}

impl<T> MatchCond<T> {
    pub fn and<U>(self, next: U) -> MatchCond<U> {
        match self {
            Self::Partial(_) => MatchCond::Partial(next),
            Self::Exact(_) => MatchCond::Exact(next),
        }
    }

    pub fn as_either(&self) -> &T {
        match self {
            Self::Partial(value) | Self::Exact(value) => value,
        }
    }

    pub fn as_match<'a, U>(&self, other: &'a MatchCond<U>) -> Option<&'a U> {
        match (self, other) {
            (Self::Partial(_), MatchCond::Partial(value)) => Some(value),
            (Self::Exact(_), MatchCond::Exact(value)) => Some(value),
            _ => None,
        }
    }

    pub fn as_partial(&self) -> Option<&T> {
        if let Self::Partial(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl<'a, T> Binding<'a, T> {
    fn new(range: Option<[usize; 2]>, nodes: Vec<MatchCond<&'a Node<T>>>) -> Self {
        Self { range, nodes }
    }

    pub fn iter(&self) -> slice::Iter<MatchCond<&'a Node<T>>> {
        self.nodes.iter()
    }

    pub fn range(&self) -> Option<[usize; 2]> {
        self.range
    }
}

impl<T> Debug for Binding<'_, T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Binding")
            .field("range", &self.range)
            .field("nodes", &self.nodes)
            .finish()
    }
}

impl<T> Node<T> {
    pub fn iter(&self) -> slice::Iter<T> {
        match self.route.as_ref() {
            Some(route) => route.iter(),
            None => [].iter(),
        }
    }

    pub fn param(&self) -> Option<&Param> {
        self.pattern.as_label()
    }
}

impl<T> Node<T> {
    fn new(pattern: Pattern) -> Self {
        Self {
            children: Either::Left(None),
            pattern,
            route: None,
        }
    }

    fn children(&self) -> &[usize] {
        match self.children.as_ref() {
            Either::Left(option) => option.as_slice(),
            Either::Right(vec) => vec.as_slice(),
        }
    }

    fn push(&mut self, key: usize) {
        let option = match &mut self.children {
            Either::Right(vec) => return vec.push(key),
            Either::Left(option) => option,
        };

        if let Some(existing) = option.take() {
            self.children = Either::Right(vec![existing, key]);
        } else {
            self.children = Either::Left(Some(key));
        }
    }
}

impl<T> Debug for Node<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[derive(Debug)]
        struct Len {
            #[allow(dead_code)]
            len: usize,
        }

        f.debug_struct("Node")
            .field("children", &self.children)
            .field("pattern", &self.pattern)
            .field(
                "route",
                &self.route.as_ref().map(|route| Len { len: route.len() }),
            )
            .finish()
    }
}

impl<T> Route<'_, T> {
    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(self.tree, &mut segments, self.key);

        Route {
            tree: self.tree,
            key,
        }
    }

    pub fn push(&mut self, middleware: T) {
        let node = &mut self.tree[self.key];
        node.route.get_or_insert_default().push(middleware);
    }
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(&mut self.tree, &mut segments, 0);

        Route {
            tree: &mut self.tree,
            key,
        }
    }

    pub fn visit<'a>(&'a self, path: &str) -> Vec<Binding<'a, T>> {
        let mut segments = Split::new(path).peekable();
        let mut results = Vec::new();
        let mut queue = SmallVec::<[usize; 6]>::new();
        let mut next = SmallVec::<[usize; 6]>::new();
        let tree = &self.tree;

        if let Some(root) = tree.first() {
            let mut nodes = Vec::with_capacity(1);

            queue.extend_from_slice(root.children());

            if segments.peek().is_none() {
                nodes.push(MatchCond::Exact(root))
            } else {
                nodes.push(MatchCond::Partial(root))
            }

            results.push(Binding::new(None, nodes));
        }

        loop {
            let mut binding = Binding::new(segments.next(), Vec::new());
            let is_last = segments.peek().is_none();
            let range = match binding.range.as_ref() {
                Some(range) => range,
                None => {
                    for key in queue.drain(..) {
                        let node = lookup!(tree, key);

                        if let Pattern::Wildcard(_) = &node.pattern {
                            binding.nodes.push(MatchCond::Exact(node));
                        }
                    }

                    if !binding.nodes.is_empty() {
                        results.push(binding);
                    }

                    return results;
                }
            };

            for key in queue.drain(..) {
                let node = lookup!(tree, key);

                match &node.pattern {
                    Pattern::Static(label) => {
                        let [start, end] = *range;

                        if label == &path[start..end] {
                            next.extend_from_slice(node.children());
                            binding.nodes.push(if is_last {
                                MatchCond::Exact(node)
                            } else {
                                MatchCond::Partial(node)
                            });
                        }
                    }

                    Pattern::Wildcard(_) => {
                        binding.nodes.push(MatchCond::Exact(node));
                    }

                    Pattern::Dynamic(_) => {
                        next.extend_from_slice(node.children());
                        binding.nodes.push(if is_last {
                            MatchCond::Exact(node)
                        } else {
                            MatchCond::Partial(node)
                        });
                    }

                    // The node does not match the range in path...
                    _ => {}
                }
            }

            if !binding.nodes.is_empty() {
                results.push(binding);
            }

            mem::swap(&mut queue, &mut next);
        }
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self {
            tree: vec![Node::new(Pattern::Root)],
        }
    }
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

    tree[parent_key].push(next_key);

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(tree, segments, next_key)
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
            let _ = router.at(path).push(());
        }

        println!("router: {:#?}", router);

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
