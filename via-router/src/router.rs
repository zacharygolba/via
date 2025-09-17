use smallvec::{IntoIter, SmallVec, smallvec};
use std::slice;
use std::sync::Arc;

use crate::path::{self, Param, Pattern, Split};

/// An iterator over the middleware for a matched route.
///
pub struct Route<'a, T>(MatchCond<slice::Iter<'a, MatchCond<T>>>);

pub struct RouteMut<'a, T>(&'a mut Node<T>);

#[derive(Debug)]
pub struct Router<T>(Node<T>);

pub struct Traverse<'a, 'b, T> {
    bindings: Vec<Binding<'a, T>>,
    queue: SmallVec<[Branch<'a, 'b, T>; 2]>,
}

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

struct Branch<'a, 'b, T> {
    children: &'a [Node<T>],
    segment: Option<(&'b str, [usize; 2])>,
    path: Split<'b>,
}

#[derive(Debug)]
struct Node<T> {
    children: Vec<Node<T>>,
    pattern: Pattern,
    route: Vec<MatchCond<T>>,
}

#[inline(always)]
fn match_next_segment<'a, 'b, T>(
    queue: &mut SmallVec<[Branch<'a, 'b, T>; 2]>,
    mut branch: Branch<'a, 'b, T>,
    segment: (&'b str, [usize; 2]),
) -> Option<Binding<'a, T>> {
    let mut results = SmallVec::new();

    let (value, range) = segment;
    let next = branch.path.next();

    for node in branch.children {
        match &node.pattern {
            Pattern::Static(name) if name != value => {}
            Pattern::Wildcard(_) => results.push(node),
            Pattern::Root => {}
            _ => {
                results.push(node);
                queue.push(Branch {
                    children: &node.children,
                    segment: next,
                    path: branch.path.clone(),
                });
            }
        }

        println!("  MATCH: {:?} ~> {}", &node.pattern, value);
    }

    if results.is_empty() {
        None
    } else {
        Some(Binding {
            is_final: next.is_none(),
            results: results.into_iter(),
            range: Some(range),
        })
    }
}

fn match_trailing_wildcards<'a, T>(branch: Branch<'a, '_, T>) -> Option<Binding<'a, T>> {
    let mut results = SmallVec::new();

    for node in branch.children {
        if let Pattern::Wildcard(_) = &node.pattern {
            results.push(node);
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

impl<'a, T> Iterator for Route<'a, T> {
    type Item = &'a T;

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

impl<T> RouteMut<'_, T> {
    pub fn at(&mut self, path: &'static str) -> RouteMut<'_, T> {
        RouteMut(insert(self.0, path::patterns(path)))
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

    pub fn at(&mut self, path: &'static str) -> RouteMut<'_, T> {
        RouteMut(insert(&mut self.0, path::patterns(path)))
    }

    /// Match the path argument against nodes in the route tree.
    ///
    /// # Panics
    ///
    /// If a node referenced by another node does not exist in the route tree.
    /// This router is insert-only, therefore this is a very unlikely scenario.
    ///
    pub fn traverse<'a, 'b>(&'a self, path: &'b str) -> Traverse<'a, 'b, T>
    where
        'a: 'b,
    {
        let Self(root) = self;

        let mut bindings = Vec::with_capacity(8);
        let mut queue = SmallVec::new();
        let mut path = Split::new(path);
        let segment = path.next();

        bindings.push(Binding {
            is_final: segment.is_none(),
            results: smallvec![root].into_iter(),
            range: None,
        });

        queue.push(Branch {
            children: &root.children,
            segment,
            path,
        });

        Traverse { bindings, queue }
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
    type Item = (Route<'a, T>, Option<(&'a Arc<str>, Param)>);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.results.next()?;
        let param = match &self.range {
            Some(range) => node.pattern.param(range),
            None => None,
        };

        if self.is_final || matches!(&node.pattern, Pattern::Wildcard(_)) {
            Some((Route(MatchCond::Final(node.route.iter())), param))
        } else {
            Some((Route(MatchCond::Partial(node.route.iter())), param))
        }
    }
}

impl<'a, 'b, T> Traverse<'a, 'b, T> {
    fn match_next(&mut self) -> bool {
        let Some(mut branch) = self.queue.pop() else {
            return false;
        };

        let next = match branch.segment.take() {
            Some(segment) => match_next_segment(&mut self.queue, branch, segment),
            None => match_trailing_wildcards(branch),
        };

        if let Some(binding) = next {
            self.bindings.push(binding);
            true
        } else {
            false
        }
    }
}

impl<'a, 'b, T> Iterator for Traverse<'a, 'b, T> {
    type Item = (Route<'a, T>, Option<(&'a Arc<str>, Param)>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(binding) = self.bindings.last_mut() else {
                if self.match_next() {
                    continue;
                } else {
                    break None;
                }
            };

            if let some @ Some(_) = binding.next() {
                self.match_next();
                break some;
            }

            self.bindings.pop();
        }
    }
}

fn insert<T, I>(node: &mut Node<T>, mut segments: I) -> &mut Node<T>
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

    use super::{Route, Router};
    use crate::path::Param;

    const PATHS: [&str; 5] = [
        "/",
        "/echo/*path",
        "/*path",
        "/articles/:id",
        "/articles/:id/comments",
    ];

    type Match<'a, N = Arc<str>> = (
        Map<Route<'a, String>, fn(&'a String) -> &'a str>,
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
        resolved: Option<(Route<'a, String>, Option<(&'a Arc<str>, Param)>)>,
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
    fn test_router_resolve() {
        let mut router = Router::new();

        for path in PATHS {
            let _ = router.at(path).include(path.to_owned());
        }

        fn assert_matches_wildcard_at_root<'a, I, F>(results: &mut I, assert_param: F)
        where
            I: Iterator<Item = (Route<'a, String>, Option<(&'a Arc<str>, Param)>)>,
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
            let mut results = router.traverse("/");

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
            let mut results = router.traverse("/not/a/path");

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
            let mut results = router.traverse("/echo/hello/world");

            assert_matches_root(expect_match(results.next()));
            assert_matches_wildcard_at_root(&mut results, |param| {
                assert_param_matches!(param, Some(("path", (1, None))))
            });

            // Intermediate match to /echo.
            {
                let (mut stack, param) = expect_match(results.next());

                println!("{:?}", stack.next());
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
            let mut results = router.traverse("/articles/12345");

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
            let mut results = router.traverse("/articles/12345/comments");

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
