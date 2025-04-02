use smallvec::SmallVec;
use std::slice;

use crate::binding::{Binding, MatchCond, MatchKind};
use crate::path::{self, Param, Pattern, Split};

#[derive(Debug)]
pub struct Node<T> {
    children: Vec<usize>,
    pattern: Pattern,
    route: Vec<MatchCond<T>>,
}

#[derive(Debug)]
pub struct Router<T> {
    tree: Vec<Node<T>>,
}

pub struct Route<'a, T> {
    tree: &'a mut Vec<Node<T>>,
    key: usize,
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

impl<T> Node<T> {
    fn new(pattern: Pattern) -> Self {
        Self {
            children: Vec::new(),
            pattern,
            route: Vec::new(),
        }
    }
}

impl<T> Node<T> {
    #[inline]
    pub fn param(&self) -> Option<&Param> {
        match &self.pattern {
            Pattern::Dynamic(param) | Pattern::Wildcard(param) => Some(param),
            _ => None,
        }
    }

    #[inline]
    pub fn route(&self) -> slice::Iter<MatchCond<T>> {
        self.route.iter()
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

    pub fn scope(&mut self, scope: impl FnOnce(&mut Self)) {
        scope(self);
    }

    pub fn include(&mut self, middleware: T) {
        let node = &mut self.tree[self.key];
        node.route.push(MatchCond::Partial(middleware));
    }

    pub fn respond(&mut self, middleware: T) {
        let node = &mut self.tree[self.key];
        node.route.push(MatchCond::Exact(middleware));
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
        let mut segments = Split::new(path).lookahead();
        let mut results = Vec::new();
        let mut branch = Vec::with_capacity(64);
        let mut next = SmallVec::<[&[usize]; 2]>::new();

        if let Some(root) = self.tree.first() {
            let mut binding = Binding::new(None, SmallVec::new());

            branch.extend_from_slice(&root.children);

            binding.push(MatchKind::edge(!segments.has_next(), root));
            results.push(binding);
        }

        while let Some((is_exact, range)) = segments.next() {
            let mut binding = Binding::new(Some(range), SmallVec::new());
            let segment = &path[range[0]..range[1]];

            for key in branch.drain(..) {
                let node = lookup!(&self.tree, key);
                let kind = match &node.pattern {
                    Pattern::Static(value) => {
                        if value == segment {
                            next.push(&node.children);
                            MatchKind::edge(is_exact, node)
                        } else {
                            continue;
                        }
                    }
                    Pattern::Wildcard(_) => MatchKind::wildcard(node),
                    Pattern::Dynamic(_) => {
                        next.push(&node.children);
                        MatchKind::edge(is_exact, node)
                    }
                    Pattern::Root => {
                        continue;
                    }
                };

                binding.push(kind);
            }

            for children in next.drain(..) {
                branch.extend_from_slice(children);
            }

            if !binding.is_empty() {
                results.push(binding);
            }
        }

        let mut wildcards = branch
            .drain(..)
            .filter_map(|key| self.match_trailing_wildcard(key))
            .peekable();

        if wildcards.peek().is_some() {
            results.push(Binding::new(None, wildcards.collect()));
        }

        results
    }

    fn match_trailing_wildcard(&self, key: usize) -> Option<MatchKind<T>> {
        let node = self.tree.get(key)?;

        if let Pattern::Wildcard(_) = &node.pattern {
            Some(MatchKind::wildcard(node))
        } else {
            None
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

    tree[parent_key].children.push(next_key);

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(tree, segments, next_key)
}

#[cfg(test)]
mod tests {
    use super::{Node, Router};
    use crate::{Binding, MatchCond, MatchKind};

    const PATHS: [&str; 5] = [
        "/",
        "/*path",
        "/echo/*path",
        "/articles/:id",
        "/articles/:id/comments",
    ];

    impl MatchCond<String> {
        fn as_str(&self) -> MatchCond<&str> {
            self.as_ref().map(|string| string.as_str())
        }
    }

    impl<T> MatchKind<'_, T> {
        fn node(&self) -> &Node<T> {
            match *self {
                Self::Edge(ref cond) => cond.as_either(),
                Self::Wildcard(node) => node,
            }
        }
    }

    fn assert_init_binding(binding: &Binding<String>, f: impl Fn(&MatchKind<String>) -> bool) {
        assert!(binding.range().is_none());
        assert_eq!(binding.nodes().count(), 1);

        let match_kind = binding.nodes().next().unwrap();

        assert_eq!(match_kind.param(), None);
        assert!(f(&match_kind));

        let mut route = match_kind.node().route();

        assert!(matches!(
            route.next().map(MatchCond::as_str),
            Some(MatchCond::Partial("/"))
        ));

        assert!(route.next().is_none());
    }

    #[test]
    fn test_router_visit() {
        let mut router = Router::new();

        for path in PATHS {
            let _ = router.at(path).include(path.to_owned());
        }

        //
        // Visit("/") [
        //     Binding(None) [
        //         Edge(Exact(Node {
        //             children: [1, 2, 4],
        //             pattern: Root,
        //             route: [Partial("/")],
        //         })),
        //     ],
        //     Binding(None) [
        //         Wildcard(Node {
        //             children: [],
        //             pattern: Wildcard(Param("path")),
        //             route: [Partial("/*path")],
        //         }),
        //     ],
        // ]
        //
        //
        {
            let results = router.visit("/");

            assert_eq!(results.len(), 2);

            assert_init_binding(results.get(0).unwrap(), |kind| {
                matches!(kind, MatchKind::Edge(MatchCond::Exact(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert!(binding.range().is_none());
                assert_eq!(binding.nodes().count(), 1);

                let match_kind = binding.nodes().next().unwrap();

                assert!(matches!(&match_kind, MatchKind::Wildcard(_)));
                assert_eq!(match_kind.param(), Some(&"path".to_owned().into()));

                let mut route = match_kind.node().route();

                assert!(matches!(
                    route.next().map(MatchCond::as_str),
                    Some(MatchCond::Partial("/*path"))
                ));

                assert!(route.next().is_none());
            }
        }

        //
        // Visit("/not/a/path") [
        //     Binding(None) [
        //         Edge(Partial(Node {
        //             children: [1, 2, 4],
        //             pattern: Root,
        //             route: [Partial("/")],
        //         })),
        //     ],
        //     Binding(Some([1, 4])) [
        //         Wildcard(Node {
        //             children: [],
        //             pattern: Wildcard(Param("path")),
        //             route: [Partial("/*path")],
        //         }),
        //     ],
        // ]
        //
        {
            let results = router.visit("/not/a/path");

            assert_eq!(results.len(), 2);

            assert_init_binding(results.get(0).unwrap(), |match_kind| {
                matches!(match_kind, MatchKind::Edge(MatchCond::Partial(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert_eq!(binding.range(), Some(&[1, 4]));
                assert_eq!(binding.nodes().count(), 1);

                let match_kind = binding.nodes().next().unwrap();

                assert!(matches!(&match_kind, MatchKind::Wildcard(_)));
                assert_eq!(match_kind.param(), Some(&"path".to_owned().into()));

                let mut route = match_kind.node().route();

                assert!(matches!(
                    route.next().map(MatchCond::as_str),
                    Some(MatchCond::Partial("/*path"))
                ));

                assert!(route.next().is_none());
            }
        }

        //
        // Visit("/echo/*path") [
        //     Binding(None) [
        //         Edge(Partial(Node {
        //             children: [1, 2, 4],
        //             pattern: Root,
        //             route: [Partial("/")],
        //         })),
        //     ],
        //     Binding(Some([1, 5])) [
        //         Wildcard(Node {
        //             children: [],
        //             pattern: Wildcard(Param("path")),
        //             route: [Partial("/*path")],
        //         }),
        //         Edge(Partial(Node {
        //             children: [3],
        //             pattern: Static("echo"),
        //             route: [],
        //         })),
        //     ],
        //     Binding(Some([6, 11])) [
        //         Wildcard(Node {
        //             children: [],
        //             pattern: Wildcard(Param("path")),
        //             route: [Partial("/echo/*path")],
        //         }),
        //     ],
        // ]
        //
        {
            let results = router.visit("/echo/hello/world");

            assert_eq!(results.len(), 3);

            assert_init_binding(results.get(0).unwrap(), |kind| {
                matches!(kind, MatchKind::Edge(MatchCond::Partial(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert_eq!(binding.range(), Some(&[1, 5]));
                assert_eq!(binding.nodes().count(), 2);

                let mut nodes = binding.nodes();

                {
                    let match_kind = nodes.next().unwrap();

                    assert!(matches!(&match_kind, MatchKind::Wildcard(_)));
                    assert_eq!(match_kind.param(), Some(&"path".to_owned().into()));

                    let node = match_kind.node();

                    assert_eq!(node.route().count(), 1);
                    assert!(matches!(
                        node.route().next().map(MatchCond::as_str),
                        Some(MatchCond::Partial("/*path"))
                    ));
                }

                {
                    let match_kind = nodes.next().unwrap();

                    assert!(matches!(
                        &match_kind,
                        MatchKind::Edge(MatchCond::Partial(_))
                    ));

                    assert!(match_kind.param().is_none());
                    assert_eq!(match_kind.node().route().count(), 0);
                }
            }

            {
                let binding = results.get(2).unwrap();

                assert_eq!(binding.range(), Some(&[6, 11]));
                assert_eq!(binding.nodes().count(), 1);

                let match_kind = binding.nodes().next().unwrap();

                assert!(matches!(&match_kind, MatchKind::Wildcard(_)));
                assert_eq!(match_kind.param(), Some(&"path".to_owned().into()));

                let node = match_kind.node();

                assert_eq!(node.route().count(), 1);
                assert!(matches!(
                    node.route().next().map(MatchCond::as_str),
                    Some(MatchCond::Partial("/echo/*path"))
                ));
            }
        }

        // Visit("/articles/8869") [
        //     Binding(None) [
        //         Edge(Partial(Node {
        //             children: [1, 2, 4],
        //             pattern: Root,
        //             route: [Partial("/")],
        //         })),
        //     ],
        //     Binding(Some([1, 9])) [
        //         Wildcard(Node {
        //             children: [],
        //             pattern: Wildcard(Param("path")),
        //             route: [Partial("/*path")],
        //         }),
        //         Edge(Partial(Node {
        //             children: [5],
        //             pattern: Static("articles"),
        //             route: [],
        //         })),
        //     ],
        //     Binding(Some([10, 14])) [
        //         Edge(Exact(Node {
        //             children: [6],
        //             pattern: Dynamic(Param("id")),
        //             route: [Partial("/articles/:id")],
        //         })),
        //     ],
        // ]

        // Visit("/articles/8869/comments") [
        //     Binding(None) [
        //         Edge(Partial(Node {
        //             children: [1, 2, 4],
        //             pattern: Root,
        //             route: [Partial("/")],
        //         })),
        //     ],
        //     Binding(Some([1, 9])) [
        //         Wildcard(Node {
        //             children: [],
        //             pattern: Wildcard(Param("path")),
        //             route: [Partial("/*path")],
        //         }),
        //         Edge(Partial(Node {
        //             children: [5],
        //             pattern: Static("articles"),
        //             route: [],
        //         })),
        //     ],
        //     Binding(Some([10, 14])) [
        //         Edge(Partial(Node {
        //             children: [6],
        //             pattern: Dynamic(Param("id")),
        //             route: [Partial("/articles/:id")],
        //         })),
        //     ],
        //     Binding(Some([15, 23])) [
        //         Edge(Exact(Node {
        //             children: [],
        //             pattern: Static("comments"),
        //             route: [Partial("/articles/:id/comments")],
        //         })),
        //     ],
        // ]
    }
}
