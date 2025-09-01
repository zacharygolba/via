use smallvec::SmallVec;
use std::{iter, mem};

use crate::binding::{Binding, Match, MatchCond};
use crate::path::{self, Param, Pattern, Split};

#[derive(Debug)]
pub struct Node<T> {
    children: Vec<Node<T>>,
    pattern: Pattern,
    route: Vec<MatchCond<T>>,
}

#[derive(Debug)]
pub struct Router<T> {
    root: Node<T>,
}

pub struct Route<'a, T> {
    node: &'a mut Node<T>,
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
    pub fn is_wildcard(&self) -> bool {
        matches!(&self.pattern, Pattern::Wildcard(_))
    }

    #[inline]
    pub fn param(&self) -> Option<&Param> {
        if let Pattern::Dynamic(name) | Pattern::Wildcard(name) = &self.pattern {
            Some(name)
        } else {
            None
        }
    }

    pub fn route(&self) -> impl Iterator<Item = &T> {
        self.route.iter().map(|cond| match cond {
            MatchCond::Partial(partial) => partial,
            MatchCond::Exact(exact) => exact,
        })
    }

    pub fn partial(&self) -> impl Iterator<Item = &T> {
        self.route.iter().filter_map(|cond| match cond {
            MatchCond::Partial(partial) => Some(partial),
            MatchCond::Exact(_) => None,
        })
    }
}

impl<T> Route<'_, T> {
    pub fn at(&mut self, path: &'static str) -> Route<'_, T> {
        let mut segments = path::patterns(path);

        Route {
            node: insert(self.node, &mut segments),
        }
    }

    pub fn scope(&mut self, scope: impl FnOnce(&mut Self)) {
        scope(self);
    }

    pub fn include(&mut self, middleware: T) {
        self.node.route.push(MatchCond::Partial(middleware));
    }

    pub fn respond(&mut self, middleware: T) {
        self.node.route.push(MatchCond::Exact(middleware));
    }
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn at(&mut self, path: &'static str) -> Route<'_, T> {
        let mut segments = path::patterns(path);

        Route {
            node: insert(&mut self.root, &mut segments),
        }
    }

    /// Match the path argument against nodes in the route tree.
    ///
    /// # Panics
    ///
    /// If a node referenced by another node does not exist in the route tree.
    /// This router is insert-only, therefore this is a very unlikely scenario.
    ///
    pub fn visit<'a, 'b>(&'a self, path: &'b str) -> impl Iterator<Item = Binding<'a, T>> + 'b
    where
        'a: 'b,
    {
        let mut root = Some(&self.root);
        let mut parents = SmallVec::<[&[Node<T>]; 2]>::new();
        let mut generation = SmallVec::<[&[Node<T>]; 2]>::new();
        let mut path_segments = Split::new(path).peekable();

        iter::from_fn(move || {
            if let Some(node) = root.take() {
                let mut binding = Binding::new(path == "/", None);

                parents.push(&node.children);
                binding.push(Match::new(&node.pattern, &node.route));

                return Some(binding);
            }

            if let Some((segment, range)) = path_segments.next() {
                let mut binding = Binding::new(path_segments.peek().is_none(), Some(range));

                for node in parents.drain(..).flatten() {
                    let children = &node.children;

                    match &node.pattern {
                        pat @ Pattern::Static(name) => {
                            if name == segment {
                                generation.push(children);
                                binding.push(Match::new(pat, &node.route));
                            } else {
                                // Placeholder for tracing...
                            }
                        }
                        pat @ Pattern::Dynamic(_) => {
                            generation.push(children);
                            binding.push(Match::new(pat, &node.route));
                        }
                        pat @ Pattern::Wildcard(_) => {
                            binding.push(Match::new(pat, &node.route));
                        }
                        Pattern::Root => {
                            // Placeholder for tracing...
                        }
                    }
                }

                mem::swap(&mut parents, &mut generation);

                return if binding.is_empty() {
                    None
                } else {
                    Some(binding)
                };
            }

            let mut wildcards = parents.drain(..).flat_map(|children| {
                children.iter().filter_map(|node| {
                    if let Pattern::Wildcard(_) = &node.pattern {
                        Some(node)
                    } else {
                        None
                    }
                })
            });

            if let Some(first) = wildcards.next() {
                let mut binding = Binding::new(true, None);

                binding.push(Match::new(&first.pattern, &first.route));
                for node in wildcards {
                    binding.push(Match::new(&node.pattern, &node.route));
                }

                Some(binding)
            } else {
                None
            }
        })
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self {
            root: Node::new(Pattern::Root),
        }
    }
}

fn insert<'a, T, I>(into: &'a mut Node<T>, segments: &mut I) -> &'a mut Node<T>
where
    I: Iterator<Item = Pattern>,
{
    let mut parent = into;

    loop {
        // If the current node is a catch-all, we can skip the rest of the segments.
        // In the future we may want to panic if the caller tries to insert a node
        // into a catch-all node rather than silently ignoring the rest of the
        // segments.
        if let Pattern::Wildcard(_) = &parent.pattern {
            return parent;
        }

        // If there are no more segments, we can return the current key.
        let pattern = match segments.next() {
            Some(value) => value,
            None => return parent,
        };

        if let Some(index) = parent
            .children
            .iter()
            .position(|node| pattern == node.pattern)
        {
            parent = &mut parent.children[index];
        } else {
            parent.children.push(Node::new(pattern));
            parent = parent.children.last_mut().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Router;
    use crate::binding::Binding;

    const PATHS: [&str; 5] = [
        "/",
        "/*path",
        "/echo/*path",
        "/articles/:id",
        "/articles/:id/comments",
    ];

    fn assert_init_binding(binding: &Binding<String>, is_final: bool) {
        assert!(binding.range().is_none());
        assert_eq!(binding.results().count(), 1);
        assert_eq!(binding.is_final(), is_final);

        let matched = binding.results().next().unwrap();

        assert_eq!(matched.param(), None);
        assert!(!matched.is_wildcard());

        let mut route = matched.exact();

        assert!(matches!(route.next().map(String::as_str), Some("/")));
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
            let results = router.visit("/").collect::<Vec<_>>();

            assert_eq!(results.len(), 2);

            assert_init_binding(results.get(0).unwrap(), true);

            {
                let binding = results.get(1).unwrap();

                assert!(binding.range().is_none());
                assert_eq!(binding.results().count(), 1);

                let matched = binding.results().next().unwrap();

                assert!(matched.is_wildcard());
                assert_eq!(matched.param(), Some(&"path".to_owned().into()));

                let mut route = matched.exact();

                assert!(matches!(route.next().map(String::as_str), Some("/*path")));

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
            let results = router.visit("/not/a/path").collect::<Vec<_>>();

            println!("{:#?}", results);
            assert_eq!(results.len(), 2);

            assert_init_binding(results.get(0).unwrap(), false);

            {
                let binding = results.get(1).unwrap();

                assert_eq!(binding.range(), Some(&[1, 4]));
                assert_eq!(binding.results().count(), 1);

                let matched = binding.results().next().unwrap();

                assert!(matched.is_wildcard());
                assert_eq!(matched.param(), Some(&"path".to_owned().into()));

                let mut route = matched.exact();

                assert!(matches!(route.next().map(String::as_str), Some("/*path")));

                assert!(route.next().is_none());
            }
        }

        //     //
        //     // Visit("/echo/*path") [
        //     //     Binding(None) [
        //     //         Edge(Partial(Node {
        //     //             children: [1, 2, 4],
        //     //             pattern: Root,
        //     //             route: [Partial("/")],
        //     //         })),
        //     //     ],
        //     //     Binding(Some([1, 5])) [
        //     //         Wildcard(Node {
        //     //             children: [],
        //     //             pattern: Wildcard(Param("path")),
        //     //             route: [Partial("/*path")],
        //     //         }),
        //     //         Edge(Partial(Node {
        //     //             children: [3],
        //     //             pattern: Static("echo"),
        //     //             route: [],
        //     //         })),
        //     //     ],
        //     //     Binding(Some([6, 11])) [
        //     //         Wildcard(Node {
        //     //             children: [],
        //     //             pattern: Wildcard(Param("path")),
        //     //             route: [Partial("/echo/*path")],
        //     //         }),
        //     //     ],
        //     // ]
        //     //
        //     {
        //         let results = router.visit("/echo/hello/world").collect::<Vec<_>>();

        //         assert_eq!(results.len(), 3);

        //         assert_init_binding(results.get(0).unwrap(), true);

        //         {
        //             let binding = results.get(1).collect::<Vec<_>>();

        //             assert_eq!(binding.range(), Some(&[1, 5]));
        //             assert_eq!(binding.results().count(), 2);

        //             let mut nodes = binding.results();

        //             {
        //                 let kind = nodes.next().collect::<Vec<_>>();

        //                 assert!(matches!(&kind, MatchKind::Wildcard(_)));
        //                 assert_eq!(kind.param(), Some(&"path".to_owned().into()));

        //                 let node = kind.node();

        //                 assert_eq!(node.route().count(), 1);
        //                 assert!(matches!(
        //                     node.route().next().map(MatchCond::as_str),
        //                     Some(MatchCond::Partial("/*path"))
        //                 ));
        //             }

        //             {
        //                 let kind = nodes.next().collect::<Vec<_>>();

        //                 assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

        //                 assert!(kind.param().is_none());

        //                 let node = kind.node();

        //                 assert_eq!(node.pattern.as_static(), Some("echo"));
        //                 assert_eq!(node.route().count(), 0);
        //             }
        //         }

        //         {
        //             let binding = results.get(2).collect::<Vec<_>>();

        //             assert_eq!(binding.range(), Some(&[6, 11]));
        //             assert_eq!(binding.results().count(), 1);

        //             let kind = binding.results().next().collect::<Vec<_>>();

        //             assert!(matches!(&kind, MatchKind::Wildcard(_)));
        //             assert_eq!(kind.param(), Some(&"path".to_owned().into()));

        //             let node = kind.node();

        //             assert_eq!(node.route().count(), 1);
        //             assert!(matches!(
        //                 node.route().next().map(MatchCond::as_str),
        //                 Some(MatchCond::Partial("/echo/*path"))
        //             ));
        //         }
        //     }

        //     // Visit("/articles/12345") [
        //     //     Binding(None) [
        //     //         Edge(Partial(Node {
        //     //             children: [1, 2, 4],
        //     //             pattern: Root,
        //     //             route: [Partial("/")],
        //     //         })),
        //     //     ],
        //     //     Binding(Some([1, 9])) [
        //     //         Wildcard(Node {
        //     //             children: [],
        //     //             pattern: Wildcard(Param("path")),
        //     //             route: [Partial("/*path")],
        //     //         }),
        //     //         Edge(Partial(Node {
        //     //             children: [5],
        //     //             pattern: Static("articles"),
        //     //             route: [],
        //     //         })),
        //     //     ],
        //     //     Binding(Some([10, 15])) [
        //     //         Edge(Exact(Node {
        //     //             children: [6],
        //     //             pattern: Dynamic(Param("id")),
        //     //             route: [Partial("/articles/:id")],
        //     //         })),
        //     //     ],
        //     // ]
        //     {
        //         let results = router.visit("/articles/12345").collect::<Vec<_>>();

        //         assert_eq!(results.len(), 3);

        //         assert_init_binding(results.get(0).unwrap(), true);

        //         {
        //             let binding = results.get(1).unwrap();

        //             assert_eq!(binding.range(), Some(&[1, 9]));
        //             assert_eq!(binding.results().count(), 2);

        //             let mut nodes = binding.results();

        //             {
        //                 let kind = nodes.next().unwrap();

        //                 assert!(matches!(&kind, MatchKind::Wildcard(_)));
        //                 assert_eq!(kind.param(), Some(&"path".to_owned().into()));

        //                 let node = kind.node();

        //                 assert_eq!(node.route().count(), 1);
        //                 assert!(matches!(
        //                     node.route().next().map(MatchCond::as_str),
        //                     Some(MatchCond::Partial("/*path"))
        //                 ));
        //             }

        //             {
        //                 let kind = nodes.next().unwrap();

        //                 assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

        //                 assert!(kind.param().is_none());

        //                 let node = kind.node();

        //                 assert_eq!(node.pattern.as_static(), Some("articles"));
        //                 assert_eq!(node.route().count(), 0);
        //             }
        //         }

        //         {
        //             let binding = results.get(2).unwrap();

        //             assert_eq!(binding.range(), Some(&[10, 15]));
        //             assert_eq!(binding.results().count(), 1);

        //             let kind = binding.results().next().unwrap();

        //             assert!(matches!(&kind, MatchKind::Edge(MatchCond::Exact(_))));
        //             assert_eq!(kind.param(), Some(&"id".to_owned().into()));

        //             let node = kind.node();

        //             assert_eq!(node.route().count(), 1);
        //             assert!(matches!(
        //                 node.route().next().map(MatchCond::as_str),
        //                 Some(MatchCond::Partial("/articles/:id"))
        //             ));
        //         }
        //     }

        //     // Visit("/articles/8869/comments") [
        //     //     Binding(None) [
        //     //         Edge(Partial(Node {
        //     //             children: [1, 2, 4],
        //     //             pattern: Root,
        //     //             route: [Partial("/")],
        //     //         })),
        //     //     ],
        //     //     Binding(Some([1, 9])) [
        //     //         Wildcard(Node {
        //     //             children: [],
        //     //             pattern: Wildcard(Param("path")),
        //     //             route: [Partial("/*path")],
        //     //         }),
        //     //         Edge(Partial(Node {
        //     //             children: [5],
        //     //             pattern: Static("articles"),
        //     //             route: [],
        //     //         })),
        //     //     ],
        //     //     Binding(Some([10, 15])) [
        //     //         Edge(Partial(Node {
        //     //             children: [6],
        //     //             pattern: Dynamic(Param("id")),
        //     //             route: [Partial("/articles/:id")],
        //     //         })),
        //     //     ],
        //     //     Binding(Some([16, 24])) [
        //     //         Edge(Exact(Node {
        //     //             children: [],
        //     //             pattern: Static("comments"),
        //     //             route: [Partial("/articles/:id/comments")],
        //     //         })),
        //     //     ],
        //     // ]
        //     {
        //         let results = router.visit("/articles/12345/comments").collect::<Vec<_>>();

        //         assert_eq!(results.len(), 4);

        //         assert_init_binding(results.get(0).unwrap(), true);

        //         {
        //             let binding = results.get(1).unwrap();

        //             assert_eq!(binding.range(), Some(&[1, 9]));
        //             assert_eq!(binding.results().count(), 2);

        //             let mut nodes = binding.results();

        //             {
        //                 let kind = nodes.next().unwrap();

        //                 assert!(matches!(&kind, MatchKind::Wildcard(_)));
        //                 assert_eq!(kind.param(), Some(&"path".to_owned().into()));

        //                 let node = kind.node();

        //                 assert_eq!(node.route().count(), 1);
        //                 assert!(matches!(
        //                     node.route().next().map(MatchCond::as_str),
        //                     Some(MatchCond::Partial("/*path"))
        //                 ));
        //             }

        //             {
        //                 let kind = nodes.next().unwrap();

        //                 assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

        //                 assert!(kind.param().is_none());

        //                 let node = kind.node();

        //                 assert_eq!(node.pattern.as_static(), Some("articles"));
        //                 assert_eq!(node.route().count(), 0);
        //             }
        //         }

        //         {
        //             let binding = results.get(2).unwrap();

        //             assert_eq!(binding.range(), Some(&[10, 15]));
        //             assert_eq!(binding.results().count(), 1);

        //             let kind = binding.results().next().unwrap();

        //             assert_eq!(kind.param(), Some(&"id".to_owned().into()));
        //             assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

        //             let node = kind.node();

        //             assert_eq!(node.route().count(), 1);
        //             assert!(matches!(
        //                 node.route().next().map(MatchCond::as_str),
        //                 Some(MatchCond::Partial("/articles/:id"))
        //             ));
        //         }

        //         {
        //             let binding = results.get(3).unwrap();

        //             assert_eq!(binding.range(), Some(&[16, 24]));
        //             assert_eq!(binding.results().count(), 1);

        //             let kind = binding.results().next().unwrap();

        //             assert_eq!(kind.param(), None);
        //             assert!(matches!(&kind, MatchKind::Edge(MatchCond::Exact(_))));

        //             let node = kind.node();

        //             assert_eq!(node.route().count(), 1);
        //             assert!(matches!(
        //                 node.route().next().map(MatchCond::as_str),
        //                 Some(MatchCond::Partial("/articles/:id/comments"))
        //             ));
        //         }
        //     }
    }
}
