use smallvec::SmallVec;
use std::slice;

use crate::binding::{Binding, MatchCond, MatchKind};
use crate::error::Error;
use crate::path::{self, Param, Pattern, Split};

/// The capacity of the vec used to store indices (usize) to the children of
/// the nodes that matched the last path segment.
///
const VISIT_BRANCH_CAPACITY: usize = 32;

/// The capacity of the vec returned from the Router::visit fn. This number is
/// calculated by assuming 7 path segments plus a binding to the root node.
///
const VISIT_RESULTS_CAPACITY: usize = 8;

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
    pub fn param<F>(&self, f: F) -> Option<(Param, [usize; 2])>
    where
        F: FnOnce() -> Option<[usize; 2]>,
    {
        if let Pattern::Dynamic(name) | Pattern::Wildcard(name) = &self.pattern {
            Some((name.clone(), f()?))
        } else {
            None
        }
    }

    #[inline]
    pub fn route(&self) -> slice::Iter<MatchCond<T>> {
        self.route.iter()
    }

    #[inline]
    pub fn matches<'node, 'predicate, U>(
        &'node self,
        predicate: &'predicate MatchCond<U>,
    ) -> impl Iterator<Item = &'node T> + 'predicate
    where
        'node: 'predicate,
    {
        self.route
            .iter()
            .filter_map(|other| predicate.matches(other.as_ref()))
    }

    #[inline]
    pub fn exact(&self) -> impl Iterator<Item = &T> {
        self.route.iter().map(MatchCond::as_either)
    }

    #[inline]
    pub fn partial(&self) -> impl Iterator<Item = &T> {
        self.route.iter().filter_map(MatchCond::as_partial)
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

    /// Match the path argument against nodes in the route tree.
    ///
    /// # Example
    ///
    /// ```
    /// use via_router::{MatchKind, Router};
    ///
    /// let mut router = Router::new();
    ///
    /// router.at("/articles").scope(|articles| {
    ///    articles.at("/:id").respond("Hello, world!".to_owned());
    /// });
    ///
    /// let path = "articles/12345";
    /// let matched = router.visit(path).unwrap().into_iter().find_map(|binding| {
    ///    let range = binding.range();
    ///
    ///    binding.nodes().find_map(|kind| match kind {
    ///       // Wildcard paths are not used in this example.
    ///       MatchKind::Wildcard(_) => None,
    ///
    ///       // The node is an exact match. Map it to the desired output and return.
    ///       MatchKind::Edge(cond) => {
    ///          let node = cond.as_either();
    ///
    ///          Some((
    ///             cond.matches(node.route().next().cloned()?)?,
    ///             node.param(|| binding.range()),
    ///          ))
    ///       }
    ///    })
    /// });
    ///
    /// if let Some((route, param)) = matched {
    ///    println!("matched {}", path);
    ///
    ///    if let Some((name, [start, end])) = param {
    ///       println!("  param: {} = {}", name, &path[start..end]);
    ///    }
    ///
    ///    println!("  => {}", route);
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// If a node referenced by another node does not exist in the route tree.
    /// This router is insert-only, therefore this is a very unlikely scenario.
    ///
    pub fn visit<'a>(&'a self, path: &str) -> Result<Vec<Binding<'a, T>>, Error> {
        let mut segments = Split::new(path).lookahead();
        let mut results = Vec::with_capacity(VISIT_RESULTS_CAPACITY);
        let mut branch = Vec::with_capacity(VISIT_BRANCH_CAPACITY);
        let mut next = SmallVec::<[&[usize]; 2]>::new();

        let tree = &self.tree;

        if let Some(root) = tree.first() {
            let mut nodes = SmallVec::new();

            branch.extend_from_slice(&root.children);

            nodes.push(MatchKind::edge(!segments.has_next(), root));
            results.push(Binding::new_with_nodes(None, nodes));
        }

        for (is_exact, range) in &mut segments {
            let mut binding = Binding::new(range);
            let segment = match path.get(range[0]..range[1]) {
                Some(value) => value,
                None => return Err(Error::path()),
            };

            {
                let mut drain = branch.drain(..);

                for key in &mut drain {
                    let node = match tree.get(key) {
                        Some(exists) => exists,
                        None => {
                            while drain.next().is_some() {}
                            return Err(Error::router());
                        }
                    };

                    binding.push(match &node.pattern {
                        Pattern::Static(value) if value == segment => {
                            next.push(&node.children);
                            MatchKind::edge(is_exact, node)
                        }
                        Pattern::Dynamic(_) => {
                            next.push(&node.children);
                            MatchKind::edge(is_exact, node)
                        }
                        Pattern::Static(_) | Pattern::Root => continue,
                        Pattern::Wildcard(_) => MatchKind::wildcard(node),
                    });
                }
            }

            for children in next.drain(..) {
                branch.extend_from_slice(children);
            }

            if binding.has_nodes() {
                results.push(binding);
            }
        }

        let mut wildcards = branch.drain(..).filter_map(|key| {
            let node = tree.get(key)?;
            if let Pattern::Wildcard(_) = &node.pattern {
                Some(MatchKind::wildcard(node))
            } else {
                None
            }
        });

        if let Some(wildcard) = wildcards.next() {
            let mut nodes = SmallVec::new();

            nodes.push(wildcard);
            nodes.extend(wildcards);
            results.push(Binding::new_with_nodes(None, nodes));
        }

        Ok(results)
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
    use crate::binding::{Binding, MatchCond, MatchKind};
    use crate::path::{Param, Pattern};

    const PATHS: [&str; 5] = [
        "/",
        "/*path",
        "/echo/*path",
        "/articles/:id",
        "/articles/:id/comments",
    ];

    impl Pattern {
        pub(crate) fn as_static(&self) -> Option<&str> {
            if let Pattern::Static(value) = self {
                Some(value)
            } else {
                None
            }
        }
    }

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

        fn param(&self) -> Option<&Param> {
            match &self.node().pattern {
                Pattern::Dynamic(name) | Pattern::Wildcard(name) => Some(name),
                Pattern::Static(_) | Pattern::Root => None,
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
            let results = router.visit("/").unwrap();

            assert_eq!(results.len(), 2);

            assert_init_binding(results.get(0).unwrap(), |kind| {
                matches!(kind, MatchKind::Edge(MatchCond::Exact(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert!(binding.range().is_none());
                assert_eq!(binding.nodes().count(), 1);

                let kind = binding.nodes().next().unwrap();

                assert!(matches!(&kind, MatchKind::Wildcard(_)));
                assert_eq!(kind.param(), Some(&"path".to_owned().into()));

                let mut route = kind.node().route();

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
            let results = router.visit("/not/a/path").unwrap();

            assert_eq!(results.len(), 2);

            assert_init_binding(results.get(0).unwrap(), |kind| {
                matches!(kind, MatchKind::Edge(MatchCond::Partial(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert_eq!(binding.range(), Some([1, 4]));
                assert_eq!(binding.nodes().count(), 1);

                let kind = binding.nodes().next().unwrap();

                assert!(matches!(&kind, MatchKind::Wildcard(_)));
                assert_eq!(kind.param(), Some(&"path".to_owned().into()));

                let mut route = kind.node().route();

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
            let results = router.visit("/echo/hello/world").unwrap();

            assert_eq!(results.len(), 3);

            assert_init_binding(results.get(0).unwrap(), |kind| {
                matches!(kind, MatchKind::Edge(MatchCond::Partial(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert_eq!(binding.range(), Some([1, 5]));
                assert_eq!(binding.nodes().count(), 2);

                let mut nodes = binding.nodes();

                {
                    let kind = nodes.next().unwrap();

                    assert!(matches!(&kind, MatchKind::Wildcard(_)));
                    assert_eq!(kind.param(), Some(&"path".to_owned().into()));

                    let node = kind.node();

                    assert_eq!(node.route().count(), 1);
                    assert!(matches!(
                        node.route().next().map(MatchCond::as_str),
                        Some(MatchCond::Partial("/*path"))
                    ));
                }

                {
                    let kind = nodes.next().unwrap();

                    assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

                    assert!(kind.param().is_none());

                    let node = kind.node();

                    assert_eq!(node.pattern.as_static(), Some("echo"));
                    assert_eq!(node.route().count(), 0);
                }
            }

            {
                let binding = results.get(2).unwrap();

                assert_eq!(binding.range(), Some([6, 11]));
                assert_eq!(binding.nodes().count(), 1);

                let kind = binding.nodes().next().unwrap();

                assert!(matches!(&kind, MatchKind::Wildcard(_)));
                assert_eq!(kind.param(), Some(&"path".to_owned().into()));

                let node = kind.node();

                assert_eq!(node.route().count(), 1);
                assert!(matches!(
                    node.route().next().map(MatchCond::as_str),
                    Some(MatchCond::Partial("/echo/*path"))
                ));
            }
        }

        // Visit("/articles/12345") [
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
        //     Binding(Some([10, 15])) [
        //         Edge(Exact(Node {
        //             children: [6],
        //             pattern: Dynamic(Param("id")),
        //             route: [Partial("/articles/:id")],
        //         })),
        //     ],
        // ]
        {
            let results = router.visit("/articles/12345").unwrap();

            assert_eq!(results.len(), 3);

            assert_init_binding(results.get(0).unwrap(), |kind| {
                matches!(kind, MatchKind::Edge(MatchCond::Partial(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert_eq!(binding.range(), Some([1, 9]));
                assert_eq!(binding.nodes().count(), 2);

                let mut nodes = binding.nodes();

                {
                    let kind = nodes.next().unwrap();

                    assert!(matches!(&kind, MatchKind::Wildcard(_)));
                    assert_eq!(kind.param(), Some(&"path".to_owned().into()));

                    let node = kind.node();

                    assert_eq!(node.route().count(), 1);
                    assert!(matches!(
                        node.route().next().map(MatchCond::as_str),
                        Some(MatchCond::Partial("/*path"))
                    ));
                }

                {
                    let kind = nodes.next().unwrap();

                    assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

                    assert!(kind.param().is_none());

                    let node = kind.node();

                    assert_eq!(node.pattern.as_static(), Some("articles"));
                    assert_eq!(node.route().count(), 0);
                }
            }

            {
                let binding = results.get(2).unwrap();

                assert_eq!(binding.range(), Some([10, 15]));
                assert_eq!(binding.nodes().count(), 1);

                let kind = binding.nodes().next().unwrap();

                assert!(matches!(&kind, MatchKind::Edge(MatchCond::Exact(_))));
                assert_eq!(kind.param(), Some(&"id".to_owned().into()));

                let node = kind.node();

                assert_eq!(node.route().count(), 1);
                assert!(matches!(
                    node.route().next().map(MatchCond::as_str),
                    Some(MatchCond::Partial("/articles/:id"))
                ));
            }
        }

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
        //     Binding(Some([10, 15])) [
        //         Edge(Partial(Node {
        //             children: [6],
        //             pattern: Dynamic(Param("id")),
        //             route: [Partial("/articles/:id")],
        //         })),
        //     ],
        //     Binding(Some([16, 24])) [
        //         Edge(Exact(Node {
        //             children: [],
        //             pattern: Static("comments"),
        //             route: [Partial("/articles/:id/comments")],
        //         })),
        //     ],
        // ]
        {
            let results = router.visit("/articles/12345/comments").unwrap();

            assert_eq!(results.len(), 4);

            assert_init_binding(results.get(0).unwrap(), |kind| {
                matches!(kind, MatchKind::Edge(MatchCond::Partial(_)))
            });

            {
                let binding = results.get(1).unwrap();

                assert_eq!(binding.range(), Some([1, 9]));
                assert_eq!(binding.nodes().count(), 2);

                let mut nodes = binding.nodes();

                {
                    let kind = nodes.next().unwrap();

                    assert!(matches!(&kind, MatchKind::Wildcard(_)));
                    assert_eq!(kind.param(), Some(&"path".to_owned().into()));

                    let node = kind.node();

                    assert_eq!(node.route().count(), 1);
                    assert!(matches!(
                        node.route().next().map(MatchCond::as_str),
                        Some(MatchCond::Partial("/*path"))
                    ));
                }

                {
                    let kind = nodes.next().unwrap();

                    assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

                    assert!(kind.param().is_none());

                    let node = kind.node();

                    assert_eq!(node.pattern.as_static(), Some("articles"));
                    assert_eq!(node.route().count(), 0);
                }
            }

            {
                let binding = results.get(2).unwrap();

                assert_eq!(binding.range(), Some([10, 15]));
                assert_eq!(binding.nodes().count(), 1);

                let kind = binding.nodes().next().unwrap();

                assert_eq!(kind.param(), Some(&"id".to_owned().into()));
                assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

                let node = kind.node();

                assert_eq!(node.route().count(), 1);
                assert!(matches!(
                    node.route().next().map(MatchCond::as_str),
                    Some(MatchCond::Partial("/articles/:id"))
                ));
            }

            {
                let binding = results.get(3).unwrap();

                assert_eq!(binding.range(), Some([16, 24]));
                assert_eq!(binding.nodes().count(), 1);

                let kind = binding.nodes().next().unwrap();

                assert_eq!(kind.param(), None);
                assert!(matches!(&kind, MatchKind::Edge(MatchCond::Exact(_))));

                let node = kind.node();

                assert_eq!(node.route().count(), 1);
                assert!(matches!(
                    node.route().next().map(MatchCond::as_str),
                    Some(MatchCond::Partial("/articles/:id/comments"))
                ));
            }
        }
    }
}
