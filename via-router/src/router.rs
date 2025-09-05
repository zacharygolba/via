use smallvec::{IntoIter, SmallVec};
use std::{iter, mem, slice, sync::Arc};

use crate::path::{self, Pattern, Split};

/// A multi-dimensional set of branches at a given depth in the route tree.
///
type Level<'a, T> = SmallVec<[&'a [Node<T>]; 2]>;

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
    results: IntoIter<[&'a Node<T>; 2]>,
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

    if results.is_empty() {
        None
    } else {
        Some(Binding {
            is_final,
            results: results.into_iter(),
            range: Some(range),
        })
    }
}

#[inline(always)]
fn match_trailing_wildcards<'a, T>(branches: &Level<'a, T>) -> Option<Binding<'a, T>> {
    let mut results = SmallVec::new();

    for branch in branches {
        for node in branch.iter() {
            if let Pattern::Wildcard(_) = &node.pattern {
                results.push(node);
            }
        }
    }

    if results.is_empty() {
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

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
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

impl<T> Router<T> {
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
    ) -> impl Iterator<Item = (Iter<'a, T>, Option<(&'a Arc<str>, (usize, Option<usize>))>)> + 'b
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
                // Unconditionally yield the root node to support middleware
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

impl<'a, T> Iterator for Binding<'a, T> {
    type Item = (Iter<'a, T>, Option<(&'a Arc<str>, (usize, Option<usize>))>);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.results.next()?;
        let param = self
            .range
            .as_ref()
            .and_then(|range| node.pattern.to_param(range));

        if self.is_final || matches!(&node.pattern, Pattern::Wildcard(_)) {
            Some((Iter(MatchCond::Final(node.route.iter())), param))
        } else {
            Some((Iter(MatchCond::Partial(node.route.iter())), param))
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
    use std::iter::Map;
    use std::sync::Arc;

    use super::{Iter, Router};
    use crate::path::Param;

    const PATHS: [&str; 5] = [
        "/",
        "/*path",
        "/echo/*path",
        "/articles/:id",
        "/articles/:id/comments",
    ];

    type Match<'a, N = Arc<str>> = (
        Map<Iter<'a, String>, fn(&'a String) -> &'a str>,
        Option<(&'a N, Param)>,
    );

    macro_rules! assert_param_matches {
        ($param:expr, $pat:pat) => {
            assert!(
                matches!($param, $pat),
                "\n{} => {:?}\n",
                stringify!($pat),
                $param
            )
        };
    }

    fn expect_match<'a>(
        resolved: Option<(Iter<'a, String>, Option<(&'a Arc<str>, Param)>)>,
    ) -> Match<'a, str> {
        if let Some((stack, param)) = resolved {
            (
                stack.map(String::as_str),
                param.map(|(name, range)| (name.as_ref(), range)),
            )
        } else {
            panic!("unexpected end of matched routes");
        }
    }

    fn assert_matches_root((mut stack, param): Match<'_, str>) {
        assert!(matches!(stack.next(), Some("/")));
        assert!(stack.next().is_none());

        assert!(param.is_none());
    }

    #[test]
    fn test_router_visit() {
        let mut router = Router::new();

        for path in PATHS {
            let _ = router.at(path).include(path.to_owned());
        }

        fn assert_matches_wildcard_at_root<'a, I, F>(results: &mut I, assert_param: F)
        where
            I: Iterator<Item = (Iter<'a, String>, Option<(&'a Arc<str>, Param)>)>,
            F: FnOnce(&Option<(&'a str, (usize, Option<usize>))>),
        {
            let (mut stack, param) = expect_match(results.next());

            assert!(matches!(stack.next(), Some("/*path")));
            assert!(stack.next().is_none());

            assert_param(&param);
        }

        //
        // visit /
        //
        // -> match "/" to root
        //     -> match "not/a/path" to Wildcard("/*path")
        //
        {
            let mut results = router.visit("/");

            assert_matches_root(expect_match(results.next()));
            assert_matches_wildcard_at_root(&mut results, |param| {
                assert_param_matches!(param, None);
            });

            assert!(results.next().is_none());
        }

        //
        // visit /not/a/path
        //
        // -> match "/" to root
        //     -> match "not/a/path" to Wildcard("/*path")
        //
        {
            let mut results = router.visit("/not/a/path");

            assert_matches_root(expect_match(results.next()));
            assert_matches_wildcard_at_root(&mut results, |param| {
                assert_param_matches!(param, Some(("path", (1, None))))
            });

            assert!(results.next().is_none());
        }

        //
        // visit /echo/hello/world
        //
        // -> match "/" to root
        //     -> match "echo/hello/world" to Wildcard("/*path")
        //     -> match "echo" to Static("echo")
        //         -> match "hello/world" to Wildcard("*path")
        //
        {
            let mut results = router.visit("/echo/hello/world");

            assert_matches_root(expect_match(results.next()));
            assert_matches_wildcard_at_root(&mut results, |param| {
                assert_param_matches!(param, Some(("path", (1, None))))
            });

            // Intermediate match to /echo.
            {
                let (mut stack, param) = expect_match(results.next());

                assert!(stack.next().is_none());
                assert!(param.is_none());
            }

            {
                let (mut stack, param) = expect_match(results.next());

                assert!(matches!(stack.next(), Some("/echo/*path")));
                assert!(stack.next().is_none());

                assert_param_matches!(param, Some(("path", (6, None))));
            }

            assert!(results.next().is_none());
        }

        //
        // visit /articles/12345
        //
        // -> match "/" to root
        //     -> match "articles/12345/comments" to Wildcard("/*path")
        //     -> match "articles" to Static("articles")
        //         -> match "12345" to Dynamic(":id")
        //
        {
            let mut results = router.visit("/articles/12345");

            assert_matches_root(expect_match(results.next()));
            assert_matches_wildcard_at_root(&mut results, |param| {
                assert_param_matches!(param, Some(("path", (1, None))))
            });

            // Intermediate match to articles.
            {
                let (mut stack, param) = expect_match(results.next());

                assert!(stack.next().is_none());
                assert_param_matches!(param, None);
            }

            {
                let (mut stack, param) = expect_match(results.next());

                assert!(matches!(stack.next(), Some("/articles/:id")));
                assert!(stack.next().is_none());

                assert_param_matches!(param, Some(("id", (10, Some(15)))));
            }

            assert!(results.next().is_none());
        }

        //
        // visit /articles/12345/comments
        //
        // -> match "/" to root
        //     -> match "articles/12345/comments" to Wildcard("/*path")
        //     -> match "articles" to Static("articles")
        //         -> match "12345" to Dynamic(":id")
        //             -> match "comments" to Static("comments")
        //
        {
            let mut results = router.visit("/articles/12345/comments");

            assert_matches_root(expect_match(results.next()));
            assert_matches_wildcard_at_root(&mut results, |param| {
                assert_param_matches!(param, Some(("path", (1, None))))
            });

            // Intermediate match to articles.
            {
                let (mut stack, param) = expect_match(results.next());

                assert!(stack.next().is_none());
                assert_param_matches!(param, None);
            }

            {
                let (mut stack, param) = expect_match(results.next());

                assert!(matches!(stack.next(), Some("/articles/:id")));
                assert!(stack.next().is_none());

                assert_param_matches!(param, Some(("id", (10, Some(15)))));
            }

            {
                let (mut stack, param) = expect_match(results.next());

                assert!(matches!(stack.next(), Some("/articles/:id/comments")));
                assert!(stack.next().is_none());

                assert_param_matches!(param, None);
            }

            assert!(results.next().is_none());
        }
    }
}
