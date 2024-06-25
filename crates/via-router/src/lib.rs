#![forbid(unsafe_code)]

mod path;
mod routes;
mod visitor;

use crate::{
    path::{Pattern, SplitPath},
    routes::{Node, RouteStore},
    visitor::Visitor,
};

pub use crate::visitor::Match;

#[derive(Debug)]
pub struct Router<T> {
    routes: RouteStore<T>,
}

#[derive(Debug)]
pub struct Endpoint<'a, T> {
    index: usize,
    routes: &'a mut RouteStore<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        let mut routes = RouteStore::new();

        routes.insert(Node::new(Pattern::Root));
        Self { routes }
    }

    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = SplitPath::new(path).into_patterns();

        Endpoint {
            index: insert(&mut self.routes, &mut segments, 0),
            routes: &mut self.routes,
        }
    }

    pub fn visit(&self, path: &str) -> Vec<Match<T>> {
        let visitor = Visitor::new(&self.routes, path);
        let root = &self.routes[0];

        visitor.visit(root)
    }
}

impl<'a, T> Endpoint<'a, T> {
    pub fn at(&mut self, path: &'static str) -> Endpoint<T> {
        let mut segments = SplitPath::new(path).into_patterns();

        Endpoint {
            index: insert(self.routes, &mut segments, self.index),
            routes: self.routes,
        }
    }

    pub fn param(&self) -> Option<&'static str> {
        let node = &self.routes[self.index];

        match node.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }

    pub fn route_mut(&mut self) -> &mut Option<T> {
        &mut self.routes[self.index].route
    }
}

fn insert<T, I>(routes: &mut RouteStore<T>, segments: &mut I, into_index: usize) -> usize
where
    I: Iterator<Item = Pattern>,
{
    // If the current node is a catch-all, we can skip the rest of the segments.
    // In the future we may want to panic if the caller tries to insert a node
    // into a catch-all node rather than silently ignoring the rest of the
    // segments.
    if let Pattern::CatchAll(_) = routes[into_index].pattern {
        while let Some(_) = segments.next() {}
        return into_index;
    }

    // If there are no more segments, we can return the current key.
    let pattern = match segments.next() {
        Some(value) => value,
        None => return into_index,
    };

    // Check if the pattern already exists in the node at `current_key`. If it does,
    // we can continue to the next segment.
    for next_index in routes[into_index].entries() {
        if pattern == routes[*next_index].pattern {
            return insert(routes, segments, *next_index);
        }
    }

    let next_index = routes.entry(into_index).insert(Node::new(pattern));

    // If the pattern does not exist in the node at `current_key`, we need to create
    // a new node as a descendant of the node at `current_key` and then insert it
    // into the store.
    insert(routes, segments, next_index)
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
            let _ = router.at(path).route_mut().insert(());
        }

        {
            let path = "/";
            let matches = router.visit(path);

            assert_eq!(matches.len(), 2);

            {
                // /
                // ^ as Pattern::Root
                let matched = &matches[0];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "");
                assert_eq!(matched.is_exact_match, true);
            }

            {
                // /
                //  ^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], "");
                // Should be considered exact because of the catch-all pattern.
                assert_eq!(matched.is_exact_match, true);
            }
        }

        {
            let path = "/not/a/path";
            let matches = router.visit(path);

            assert_eq!(matches.len(), 2);

            {
                // /not/a/path
                // ^ as Pattern::Root
                let matched = &matches[0];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /not/a/path
                //  ^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert_eq!(matched.is_exact_match, true);
            }
        }

        {
            let path = "/echo/hello/world";
            let matches = router.visit(path);

            assert_eq!(matches.len(), 4);

            {
                // /echo/hello/world
                // ^ as Pattern::Root
                let matched = &matches[0];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /echo/hello/world
                //  ^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert_eq!(matched.is_exact_match, true);
            }

            {
                // /echo/hello/world
                //  ^^^^ as Pattern::Static("echo")
                let matched = &matches[2];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "echo");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /echo/hello/world
                //       ^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[3];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], "hello/world");
                assert_eq!(matched.is_exact_match, true);
            }
        }

        {
            let path = "/articles/100";
            let matches = router.visit(path);

            assert_eq!(matches.len(), 4);

            {
                // /articles/100
                // ^ as Pattern::Root
                let matched = &matches[0];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /articles/100
                //  ^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert_eq!(matched.is_exact_match, true);
            }

            {
                // /articles/100
                //  ^^^^^^^^ as Pattern::Static("articles")
                let matched = &matches[2];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "articles");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /articles/100
                //           ^^^ as Pattern::Dynamic(":id")
                let matched = &matches[3];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], "100");
                assert_eq!(matched.is_exact_match, true);
            }
        }

        {
            let path = "/articles/100/comments";
            let matches = router.visit(path);

            assert_eq!(matches.len(), 5);

            {
                // /articles/100/comments
                // ^ as Pattern::Root
                let matched = &matches[0];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^^^^^^^^^^^^^^ as Pattern::CatchAll("*path")
                let matched = &matches[1];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], &path[1..]);
                // Should be considered exact because of the catch-all pattern.
                assert_eq!(matched.is_exact_match, true);
            }

            {
                // /articles/100/comments
                //  ^^^^^^^^ as Pattern::Static("articles")
                let matched = &matches[2];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), None);
                assert_eq!(&path[start..end], "articles");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /articles/100/comments
                //           ^^^ as Pattern::Dynamic(":id")
                let matched = &matches[3];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], "100");
                assert_eq!(matched.is_exact_match, false);
            }

            {
                // /articles/100/comments
                //               ^^^^^^^^ as Pattern::Static("comments")
                let matched = &matches[4];
                let (start, end) = matched.path_segment_range;

                assert_eq!(matched.route(), Some(&()));
                assert_eq!(&path[start..end], "comments");
                // Should be considered exact because it is the last path segment.
                assert_eq!(matched.is_exact_match, true);
            }
        }
    }
}
