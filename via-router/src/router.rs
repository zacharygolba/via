use smallvec::{IntoIter, SmallVec};
use std::{iter, mem, slice};

use crate::path::{self, Pattern, Split};

/// A multi-dimensional set of branches at a given depth in the route tree.
///
type Level<'a, T> = SmallVec<[&'a [Node<T>]; 1]>;

/// An iterator over the middleware for a matched route.
///
pub struct Iter<'a, T>(MatchCond<slice::Iter<'a, MatchCond<T>>>);

pub struct Route<'a, T>(&'a mut Node<T>);

#[derive(Debug)]
pub struct Router<T>(Node<T>);

#[derive(Debug)]
enum MatchCond<T> {
    Partial(T),
    Final(T),
}

/// A group of nodes that match the path segment at `self.range`.
///
struct Binding<'a, T> {
    is_final: bool,
    results: IntoIter<[&'a Node<T>; 1]>,
    range: Option<[usize; 2]>,
}

#[derive(Debug)]
struct Node<T> {
    children: Vec<Node<T>>,
    pattern: Pattern,
    route: Vec<MatchCond<T>>,
}

#[inline(always)]
fn match_next_segment<'a, T>(
    is_final: bool,
    queue: &mut Level<'a, T>,
    branches: &Level<'a, T>,
    segment: &str,
    range: [usize; 2],
) -> Option<Binding<'a, T>> {
    let mut results = SmallVec::new();

    for branch in branches {
        for node in branch.iter() {
            match &node.pattern {
                Pattern::Static(name) if name == segment => {
                    queue.push(&node.children);
                    results.push(node);
                }
                Pattern::Dynamic(_) => {
                    queue.push(&node.children);
                    results.push(node);
                }
                Pattern::Wildcard(_) => {
                    results.push(node);
                }
                Pattern::Static(_) => {
                    // The node does not match the path segment.
                }
                Pattern::Root => {
                    // The root node was matched as a descendant of a child node.
                    // Either an error occurred during the construction of the
                    // route tree or the memory where the route tree is stored
                    // became corrupt.
                }
            }
        }
    }

    Some(Binding {
        is_final,
        results: results.into_iter(),
        range: Some(range),
    })
}

#[inline(always)]
fn match_trailing_wildcards<'a, T>(branches: &Level<'a, T>) -> Option<Binding<'a, T>> {
    let mut results = SmallVec::new();
    let mut empty = true;

    for branch in branches {
        for node in branch.iter() {
            if let Pattern::Wildcard(_) = &node.pattern {
                empty = false;
                results.push(node);
            }
        }
    }

    if empty {
        None
    } else {
        Some(Binding {
            is_final: true,
            results: results.into_iter(),
            range: None,
        })
    }
}

impl<T> MatchCond<T> {
    #[inline]
    fn as_either(&self) -> &T {
        match self {
            Self::Final(value) | Self::Partial(value) => value,
        }
    }

    #[inline]
    fn as_partial(&self) -> Option<&T> {
        if let Self::Partial(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl<'a, T, U> Iterator for MatchCond<T>
where
    T: Iterator<Item = &'a MatchCond<U>>,
    U: 'a,
{
    type Item = &'a U;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            break match self {
                MatchCond::Final(iter) => iter.next().map(MatchCond::as_either),
                MatchCond::Partial(iter) => match iter.next()?.as_partial() {
                    None => continue,
                    some => some,
                },
            };
        }
    }
}

impl<'a, T: Clone> Iterator for Iter<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().cloned()
    }
}

impl<T> Node<T> {
    fn push(&mut self, pattern: Pattern) -> &mut Node<T> {
        let children = &mut self.children;
        let index = children.len();

        children.push(Node {
            children: Vec::new(),
            pattern,
            route: Vec::new(),
        });

        &mut children[index]
    }
}

impl<T> Route<'_, T> {
    pub fn at(&mut self, path: &'static str) -> Route<'_, T> {
        Route(insert(self.0, path::patterns(path)))
    }

    pub fn scope(&mut self, scope: impl FnOnce(&mut Self)) {
        scope(self);
    }

    pub fn include(&mut self, middleware: T) {
        self.0.route.push(MatchCond::Partial(middleware));
    }

    pub fn respond(&mut self, middleware: T) {
        self.0.route.push(MatchCond::Final(middleware));
    }
}

impl<T: Clone> Router<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn at(&mut self, path: &'static str) -> Route<'_, T> {
        Route(insert(&mut self.0, path::patterns(path)))
    }

    /// Match the path argument against nodes in the route tree.
    ///
    /// # Panics
    ///
    /// If a node referenced by another node does not exist in the route tree.
    /// This router is insert-only, therefore this is a very unlikely scenario.
    ///
    pub fn visit<'a, 'b>(
        &'a self,
        path: &'b str,
    ) -> impl Iterator<Item = (Iter<'a, T>, Option<(String, (usize, Option<usize>))>)> + 'b
    where
        'a: 'b,
    {
        // Keep a reference to the root node live for the sake of correctness
        // and consistency with graph / tree-like algorithms.
        let root = &self.0;

        // An option containing a reference to the root node. We use this to
        // yield the first binding and seed the iterator with nodes to match
        // against subsequent path segments.
        let mut entrypoint = Some(root);

        // A multi-dimensional vec that can store a single branch inline to
        // match against the current path segment.
        let mut branches = Level::new();

        // Same as `branches` but used to accumulate branches to match against
        // the next path segment. This value is swapped with `branches` during
        // each iteration.
        let mut queue = Level::new();

        // A peekable iterator that yields the next path segment and the range
        // at which it can be found in path. We have to peek in order to
        // determine if a binding is "final".
        let mut split = Split::new(path).peekable();

        Iterator::flatten(iter::from_fn(move || {
            // We'll need this pointer at least once and also in 2 of the 3
            // possible branches that will run before the next binding is
            // returned
            let branches = &mut branches;

            // Same as `branches`, we'll likely use this pointer twice. We're
            // better of building the reference early.
            let queue = &mut queue;

            // An optional Binding that contains nodes at the current depth
            // that match the next path segment.
            let next = entrypoint
                .take()
                // Unconditional yield the root node to support middleware
                // functions that are applied to the entire route stack.
                .map(|node| {
                    let mut results = SmallVec::new();

                    queue.push(&node.children);
                    results.push(node);

                    Binding {
                        is_final: path == "/",
                        results: results.into_iter(),
                        range: None,
                    }
                })
                // We already yielded a binding to the root node. Advance to
                // the next path segment.
                .or_else(|| match split.next() {
                    // Match the nodes at the current level against the current
                    // path segment. Then, add the children of each matching
                    // node to the queue to match against the next path
                    // segment.
                    Some((segment, range)) => {
                        match_next_segment(split.peek().is_none(), queue, branches, segment, range)
                    }
                    // There are no more path segments to match against. Search
                    // for nodes at the current level with a wildcard pattern
                    // to support optional path parameters for wildcard nodes.
                    //
                    // This is a pattern that is commonly used when serving
                    // static HTML pages.
                    //
                    // i.e request.param("path").unwrap_or("index.html");
                    None => match_trailing_wildcards(branches),
                });

            branches.clear();

            // Swap the current level with the queue to source matching nodes
            // during the next iteration. Any allocations that were made at
            // this depth will be reused.
            mem::swap(branches, queue);

            next
        }))
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self(Node {
            children: Vec::new(),
            pattern: Pattern::Root,
            route: Vec::new(),
        })
    }
}

impl<'a, T: Clone> Iterator for Binding<'a, T> {
    type Item = (Iter<'a, T>, Option<(String, (usize, Option<usize>))>);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.results.next()?;

        match &node.pattern {
            Pattern::Wildcard(name) => Some((
                Iter(MatchCond::Final(node.route.iter())),
                self.range.map(|[start, _]| (name.clone(), (start, None))),
            )),
            Pattern::Dynamic(name) => {
                let param = Some(name.clone()).zip(self.range.map(|[s, e]| (s, Some(e))));

                Some(if self.is_final {
                    (Iter(MatchCond::Final(node.route.iter())), param)
                } else {
                    (Iter(MatchCond::Partial(node.route.iter())), param)
                })
            }
            _ => Some(if self.is_final {
                (Iter(MatchCond::Final(node.route.iter())), None)
            } else {
                (Iter(MatchCond::Partial(node.route.iter())), None)
            }),
        }
    }
}

fn insert<'a, T, I>(node: &'a mut Node<T>, mut segments: I) -> &'a mut Node<T>
where
    I: Iterator<Item = Pattern>,
{
    let mut parent = node;

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

        parent = if let Some(index) = parent
            .children
            .iter()
            .position(|node| pattern == node.pattern)
        {
            &mut parent.children[index]
        } else {
            parent.push(pattern)
        };
    }
}

#[cfg(test)]
mod tests {
    // use super::{Binding, Router};

    // const PATHS: [&str; 5] = [
    //     "/",
    //     "/*path",
    //     "/echo/*path",
    //     "/articles/:id",
    //     "/articles/:id/comments",
    // ];

    // fn assert_init_binding(binding: &Binding<String>, is_final: bool) {
    //     assert!(binding.range().is_none());
    //     assert_eq!(binding.results().count(), 1);
    //     assert_eq!(binding.is_final(), is_final);

    //     let matched = binding.results().next().unwrap();

    //     assert_eq!(matched.param(), None);
    //     assert!(!matched.is_wildcard());

    //     let mut route = matched.as_final();

    //     assert!(matches!(route.next().map(String::as_str), Some("/")));
    //     assert!(route.next().is_none());
    // }

    // #[test]
    // fn test_router_visit() {
    //     let mut router = Router::new();

    //     for path in PATHS {
    //         let _ = router.at(path).include(path.to_owned());
    //     }

    //     //
    //     // Visit("/") [
    //     //     Binding(None) [
    //     //         Edge(Exact(Node {
    //     //             children: [1, 2, 4],
    //     //             pattern: Root,
    //     //             route: [Partial("/")],
    //     //         })),
    //     //     ],
    //     //     Binding(None) [
    //     //         Wildcard(Node {
    //     //             children: [],
    //     //             pattern: Wildcard(Param("path")),
    //     //             route: [Partial("/*path")],
    //     //         }),
    //     //     ],
    //     // ]
    //     //
    //     //
    //     {
    //         let results = router.visit("/").collect::<Vec<_>>();

    //         assert_eq!(results.len(), 2);

    //         assert_init_binding(results.get(0).unwrap(), true);

    //         {
    //             let binding = results.get(1).unwrap();

    //             assert!(binding.range().is_none());
    //             assert_eq!(binding.results().count(), 1);

    //             let matched = binding.results().next().unwrap();

    //             assert!(matched.is_wildcard());
    //             assert_eq!(matched.param(), Some(&"path".to_owned().into()));

    //             let mut route = matched.as_final();

    //             assert!(matches!(route.next().map(String::as_str), Some("/*path")));

    //             assert!(route.next().is_none());
    //         }
    //     }

    //     //
    //     // Visit("/not/a/path") [
    //     //     Binding(None) [
    //     //         Edge(Partial(Node {
    //     //             children: [1, 2, 4],
    //     //             pattern: Root,
    //     //             route: [Partial("/")],
    //     //         })),
    //     //     ],
    //     //     Binding(Some([1, 4])) [
    //     //         Wildcard(Node {
    //     //             children: [],
    //     //             pattern: Wildcard(Param("path")),
    //     //             route: [Partial("/*path")],
    //     //         }),
    //     //     ],
    //     // ]
    //     //
    //     {
    //         let results = router.visit("/not/a/path").collect::<Vec<_>>();

    //         println!("{:#?}", results);
    //         assert_eq!(results.len(), 2);

    //         assert_init_binding(results.get(0).unwrap(), false);

    //         {
    //             let binding = results.get(1).unwrap();

    //             assert_eq!(binding.range(), Some(&[1, 4]));
    //             assert_eq!(binding.results().count(), 1);

    //             let matched = binding.results().next().unwrap();

    //             assert!(matched.is_wildcard());
    //             assert_eq!(matched.param(), Some(&"path".to_owned().into()));

    //             let mut route = matched.as_final();

    //             assert!(matches!(route.next().map(String::as_str), Some("/*path")));

    //             assert!(route.next().is_none());
    //         }
    //     }

    //     //     //
    //     //     // Visit("/echo/*path") [
    //     //     //     Binding(None) [
    //     //     //         Edge(Partial(Node {
    //     //     //             children: [1, 2, 4],
    //     //     //             pattern: Root,
    //     //     //             route: [Partial("/")],
    //     //     //         })),
    //     //     //     ],
    //     //     //     Binding(Some([1, 5])) [
    //     //     //         Wildcard(Node {
    //     //     //             children: [],
    //     //     //             pattern: Wildcard(Param("path")),
    //     //     //             route: [Partial("/*path")],
    //     //     //         }),
    //     //     //         Edge(Partial(Node {
    //     //     //             children: [3],
    //     //     //             pattern: Static("echo"),
    //     //     //             route: [],
    //     //     //         })),
    //     //     //     ],
    //     //     //     Binding(Some([6, 11])) [
    //     //     //         Wildcard(Node {
    //     //     //             children: [],
    //     //     //             pattern: Wildcard(Param("path")),
    //     //     //             route: [Partial("/echo/*path")],
    //     //     //         }),
    //     //     //     ],
    //     //     // ]
    //     //     //
    //     //     {
    //     //         let results = router.visit("/echo/hello/world").collect::<Vec<_>>();

    //     //         assert_eq!(results.len(), 3);

    //     //         assert_init_binding(results.get(0).unwrap(), true);

    //     //         {
    //     //             let binding = results.get(1).collect::<Vec<_>>();

    //     //             assert_eq!(binding.range(), Some(&[1, 5]));
    //     //             assert_eq!(binding.results().count(), 2);

    //     //             let mut nodes = binding.results();

    //     //             {
    //     //                 let kind = nodes.next().collect::<Vec<_>>();

    //     //                 assert!(matches!(&kind, MatchKind::Wildcard(_)));
    //     //                 assert_eq!(kind.param(), Some(&"path".to_owned().into()));

    //     //                 let node = kind.node();

    //     //                 assert_eq!(node.route().count(), 1);
    //     //                 assert!(matches!(
    //     //                     node.route().next().map(MatchCond::as_str),
    //     //                     Some(MatchCond::Partial("/*path"))
    //     //                 ));
    //     //             }

    //     //             {
    //     //                 let kind = nodes.next().collect::<Vec<_>>();

    //     //                 assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

    //     //                 assert!(kind.param().is_none());

    //     //                 let node = kind.node();

    //     //                 assert_eq!(node.pattern.as_static(), Some("echo"));
    //     //                 assert_eq!(node.route().count(), 0);
    //     //             }
    //     //         }

    //     //         {
    //     //             let binding = results.get(2).collect::<Vec<_>>();

    //     //             assert_eq!(binding.range(), Some(&[6, 11]));
    //     //             assert_eq!(binding.results().count(), 1);

    //     //             let kind = binding.results().next().collect::<Vec<_>>();

    //     //             assert!(matches!(&kind, MatchKind::Wildcard(_)));
    //     //             assert_eq!(kind.param(), Some(&"path".to_owned().into()));

    //     //             let node = kind.node();

    //     //             assert_eq!(node.route().count(), 1);
    //     //             assert!(matches!(
    //     //                 node.route().next().map(MatchCond::as_str),
    //     //                 Some(MatchCond::Partial("/echo/*path"))
    //     //             ));
    //     //         }
    //     //     }

    //     //     // Visit("/articles/12345") [
    //     //     //     Binding(None) [
    //     //     //         Edge(Partial(Node {
    //     //     //             children: [1, 2, 4],
    //     //     //             pattern: Root,
    //     //     //             route: [Partial("/")],
    //     //     //         })),
    //     //     //     ],
    //     //     //     Binding(Some([1, 9])) [
    //     //     //         Wildcard(Node {
    //     //     //             children: [],
    //     //     //             pattern: Wildcard(Param("path")),
    //     //     //             route: [Partial("/*path")],
    //     //     //         }),
    //     //     //         Edge(Partial(Node {
    //     //     //             children: [5],
    //     //     //             pattern: Static("articles"),
    //     //     //             route: [],
    //     //     //         })),
    //     //     //     ],
    //     //     //     Binding(Some([10, 15])) [
    //     //     //         Edge(Exact(Node {
    //     //     //             children: [6],
    //     //     //             pattern: Dynamic(Param("id")),
    //     //     //             route: [Partial("/articles/:id")],
    //     //     //         })),
    //     //     //     ],
    //     //     // ]
    //     //     {
    //     //         let results = router.visit("/articles/12345").collect::<Vec<_>>();

    //     //         assert_eq!(results.len(), 3);

    //     //         assert_init_binding(results.get(0).unwrap(), true);

    //     //         {
    //     //             let binding = results.get(1).unwrap();

    //     //             assert_eq!(binding.range(), Some(&[1, 9]));
    //     //             assert_eq!(binding.results().count(), 2);

    //     //             let mut nodes = binding.results();

    //     //             {
    //     //                 let kind = nodes.next().unwrap();

    //     //                 assert!(matches!(&kind, MatchKind::Wildcard(_)));
    //     //                 assert_eq!(kind.param(), Some(&"path".to_owned().into()));

    //     //                 let node = kind.node();

    //     //                 assert_eq!(node.route().count(), 1);
    //     //                 assert!(matches!(
    //     //                     node.route().next().map(MatchCond::as_str),
    //     //                     Some(MatchCond::Partial("/*path"))
    //     //                 ));
    //     //             }

    //     //             {
    //     //                 let kind = nodes.next().unwrap();

    //     //                 assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

    //     //                 assert!(kind.param().is_none());

    //     //                 let node = kind.node();

    //     //                 assert_eq!(node.pattern.as_static(), Some("articles"));
    //     //                 assert_eq!(node.route().count(), 0);
    //     //             }
    //     //         }

    //     //         {
    //     //             let binding = results.get(2).unwrap();

    //     //             assert_eq!(binding.range(), Some(&[10, 15]));
    //     //             assert_eq!(binding.results().count(), 1);

    //     //             let kind = binding.results().next().unwrap();

    //     //             assert!(matches!(&kind, MatchKind::Edge(MatchCond::Exact(_))));
    //     //             assert_eq!(kind.param(), Some(&"id".to_owned().into()));

    //     //             let node = kind.node();

    //     //             assert_eq!(node.route().count(), 1);
    //     //             assert!(matches!(
    //     //                 node.route().next().map(MatchCond::as_str),
    //     //                 Some(MatchCond::Partial("/articles/:id"))
    //     //             ));
    //     //         }
    //     //     }

    //     //     // Visit("/articles/8869/comments") [
    //     //     //     Binding(None) [
    //     //     //         Edge(Partial(Node {
    //     //     //             children: [1, 2, 4],
    //     //     //             pattern: Root,
    //     //     //             route: [Partial("/")],
    //     //     //         })),
    //     //     //     ],
    //     //     //     Binding(Some([1, 9])) [
    //     //     //         Wildcard(Node {
    //     //     //             children: [],
    //     //     //             pattern: Wildcard(Param("path")),
    //     //     //             route: [Partial("/*path")],
    //     //     //         }),
    //     //     //         Edge(Partial(Node {
    //     //     //             children: [5],
    //     //     //             pattern: Static("articles"),
    //     //     //             route: [],
    //     //     //         })),
    //     //     //     ],
    //     //     //     Binding(Some([10, 15])) [
    //     //     //         Edge(Partial(Node {
    //     //     //             children: [6],
    //     //     //             pattern: Dynamic(Param("id")),
    //     //     //             route: [Partial("/articles/:id")],
    //     //     //         })),
    //     //     //     ],
    //     //     //     Binding(Some([16, 24])) [
    //     //     //         Edge(Exact(Node {
    //     //     //             children: [],
    //     //     //             pattern: Static("comments"),
    //     //     //             route: [Partial("/articles/:id/comments")],
    //     //     //         })),
    //     //     //     ],
    //     //     // ]
    //     //     {
    //     //         let results = router.visit("/articles/12345/comments").collect::<Vec<_>>();

    //     //         assert_eq!(results.len(), 4);

    //     //         assert_init_binding(results.get(0).unwrap(), true);

    //     //         {
    //     //             let binding = results.get(1).unwrap();

    //     //             assert_eq!(binding.range(), Some(&[1, 9]));
    //     //             assert_eq!(binding.results().count(), 2);

    //     //             let mut nodes = binding.results();

    //     //             {
    //     //                 let kind = nodes.next().unwrap();

    //     //                 assert!(matches!(&kind, MatchKind::Wildcard(_)));
    //     //                 assert_eq!(kind.param(), Some(&"path".to_owned().into()));

    //     //                 let node = kind.node();

    //     //                 assert_eq!(node.route().count(), 1);
    //     //                 assert!(matches!(
    //     //                     node.route().next().map(MatchCond::as_str),
    //     //                     Some(MatchCond::Partial("/*path"))
    //     //                 ));
    //     //             }

    //     //             {
    //     //                 let kind = nodes.next().unwrap();

    //     //                 assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

    //     //                 assert!(kind.param().is_none());

    //     //                 let node = kind.node();

    //     //                 assert_eq!(node.pattern.as_static(), Some("articles"));
    //     //                 assert_eq!(node.route().count(), 0);
    //     //             }
    //     //         }

    //     //         {
    //     //             let binding = results.get(2).unwrap();

    //     //             assert_eq!(binding.range(), Some(&[10, 15]));
    //     //             assert_eq!(binding.results().count(), 1);

    //     //             let kind = binding.results().next().unwrap();

    //     //             assert_eq!(kind.param(), Some(&"id".to_owned().into()));
    //     //             assert!(matches!(&kind, MatchKind::Edge(MatchCond::Partial(_))));

    //     //             let node = kind.node();

    //     //             assert_eq!(node.route().count(), 1);
    //     //             assert!(matches!(
    //     //                 node.route().next().map(MatchCond::as_str),
    //     //                 Some(MatchCond::Partial("/articles/:id"))
    //     //             ));
    //     //         }

    //     //         {
    //     //             let binding = results.get(3).unwrap();

    //     //             assert_eq!(binding.range(), Some(&[16, 24]));
    //     //             assert_eq!(binding.results().count(), 1);

    //     //             let kind = binding.results().next().unwrap();

    //     //             assert_eq!(kind.param(), None);
    //     //             assert!(matches!(&kind, MatchKind::Edge(MatchCond::Exact(_))));

    //     //             let node = kind.node();

    //     //             assert_eq!(node.route().count(), 1);
    //     //             assert!(matches!(
    //     //                 node.route().next().map(MatchCond::as_str),
    //     //                 Some(MatchCond::Partial("/articles/:id/comments"))
    //     //             ));
    //     //         }
    //     //     }
    // }
}
