use smallvec::SmallVec;
use std::slice;
use std::sync::Arc;

use crate::path::{self, Param, Pattern, Split};

#[derive(Debug)]
pub enum MatchCond<T> {
    Partial(T),
    Exact(T),
}

#[derive(Debug)]
pub struct Binding<T> {
    is_exact: bool,
    offset: usize,
    node: Arc<Node<T>>,
    to: Option<[usize; 2]>,
}

pub struct Builder<T> {
    tree: Vec<Node<T>>,
}

#[derive(Debug)]
pub struct Router<T> {
    tree: Vec<Arc<Node<T>>>,
}

pub struct Route<'a, T> {
    tree: &'a mut Vec<Node<T>>,
    key: usize,
}

#[derive(Debug)]
pub struct Node<T> {
    children: Vec<usize>,
    pattern: Pattern,
    route: Vec<MatchCond<T>>,
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
    for key in tree[parent_key].iter().copied() {
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

impl<T> Binding<T> {
    fn new(is_exact: bool, node: Arc<Node<T>>, to: Option<[usize; 2]>) -> Self {
        Self {
            is_exact,
            offset: 0,
            node,
            to,
        }
    }

    pub fn param(&self) -> Option<(&Param, Option<[usize; 2]>)> {
        self.node.param().map(|name| (name, self.to))
    }

    pub fn next(&mut self) -> Option<usize> {
        let is_exact = self.is_exact;
        let offset = &mut self.offset;
        let route = &self.node.route;

        loop {
            let key = *offset;
            return match route.get(key)? {
                MatchCond::Exact(_) if is_exact => {
                    *offset += 1;
                    Some(key)
                }
                MatchCond::Partial(_) => {
                    *offset += 1;
                    Some(key)
                }
                MatchCond::Exact(_) => {
                    *offset += 1;
                    continue;
                }
            };
        }
    }

    pub fn get(&self, key: usize) -> Option<&T> {
        self.node.route.get(key).map(MatchCond::as_either)
    }
}

impl<T> Builder<T> {
    pub fn at(&mut self, path: &'static str) -> Route<T> {
        let mut segments = path::patterns(path);
        let key = insert(&mut self.tree, &mut segments, 0);

        Route {
            tree: &mut self.tree,
            key,
        }
    }

    pub fn build(self) -> Router<T> {
        Router {
            tree: self.tree.into_iter().map(Arc::new).collect(),
        }
    }
}

impl<T> Router<T> {
    pub fn build() -> Builder<T> {
        Builder {
            tree: vec![Node::new(Pattern::Root)],
        }
    }

    pub fn visit(&self, path: &str) -> Vec<Binding<T>> {
        let mut segments = Split::new(path).peekable();
        let mut results = Vec::new();
        let mut queue = SmallVec::<[usize; 4]>::new();
        let mut next = SmallVec::<[usize; 4]>::new();
        let tree = &self.tree;

        if let Some(root) = tree.first() {
            queue.extend_from_slice(&root.children);
            results.push(Binding::new(segments.peek().is_none(), root.clone(), None));
        }

        while let Some(range) = segments.next() {
            let segment = &path[range[0]..range[1]];
            let is_exact = segments.peek().is_none();

            for key in queue.drain(..) {
                let node = match tree.get(key) {
                    Some(next) => next,
                    None => {
                        // Placeholder for tracing...
                        continue;
                    }
                };

                match node.pattern() {
                    Pattern::Static(value) if value == segment => {
                        next.extend_from_slice(&node.children);
                        results.push(Binding::new(is_exact, node.clone(), Some(range)));
                    }

                    Pattern::Wildcard(_) => {
                        let [start, _] = range;
                        results.push(Binding::new(true, node.clone(), Some([start, path.len()])));
                    }

                    Pattern::Dynamic(_) => {
                        next.extend_from_slice(&node.children);
                        results.push(Binding::new(is_exact, node.clone(), Some(range)));
                    }

                    _ => {}
                }
            }

            queue.extend(next.drain(..));
        }

        for key in queue.drain(..) {
            let node = match tree.get(key) {
                Some(next) => next,
                None => {
                    // Placeholder for tracing...
                    continue;
                }
            };

            if let Pattern::Wildcard(_) = node.pattern() {
                results.push(Binding::new(true, node.clone(), None));
            }
        }

        results
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

    pub fn push(&mut self, middleware: MatchCond<T>) {
        self.tree[self.key].route.push(middleware);
    }
}

impl<T> MatchCond<T> {
    fn as_either(&self) -> &T {
        match self {
            Self::Exact(route) | Self::Partial(route) => route,
        }
    }

    fn as_partial(&self) -> Option<&T> {
        if let Self::Partial(route) = self {
            Some(route)
        } else {
            None
        }
    }
}

impl<T> Node<T> {
    fn new(pattern: Pattern) -> Self {
        Self {
            children: Vec::new(),
            pattern,
            route: Vec::new(),
        }
    }

    fn iter(&self) -> slice::Iter<usize> {
        self.children.iter()
    }

    fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    fn param(&self) -> Option<&Param> {
        match &self.pattern {
            Pattern::Dynamic(param) | Pattern::Wildcard(param) => Some(param),
            Pattern::Root | Pattern::Static(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MatchCond, Router};

    const PATHS: [&str; 4] = [
        "/*path",
        "/echo/*path",
        "/articles/:id",
        "/articles/:id/comments",
    ];

    #[test]
    fn test_router_visit() {
        let router = {
            let mut builder = Router::build();

            for path in &PATHS {
                let _ = builder.at(path).push(MatchCond::Exact(()));
            }

            builder.build()
        };

        println!("router: {:?}", router);

        {
            let path = "/";
            let matches: Vec<_> = router.visit(path);

            println!("match / {:?}", matches);
            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let binding = &matches[0];

                assert!(binding.is_exact);

                assert_eq!(binding.to, None);
                assert_eq!(binding.param(), None);
                assert_eq!(binding.node.route.len(), 0);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let binding = &matches[1];

                assert_eq!(binding.to, None);
                assert_eq!(binding.node.route.len(), 1);
                assert_eq!(binding.param(), Some((&"path".into(), None)));
                // Should be considered exact because of the catch-all pattern.
                assert!(binding.is_exact);
            }
        }

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
