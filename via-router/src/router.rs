use smallvec::SmallVec;
use std::fmt::{self, Debug, Formatter};

use crate::binding::{Binding, Match, MatchCond};
use crate::path::{self, Pattern, Split};

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

impl<T> Debug for Node<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[derive(Debug)]
        struct Route {
            #[allow(dead_code)]
            len: usize,
        }

        f.debug_struct("Node")
            .field("children", &self.children)
            .field("pattern", &self.pattern)
            .field(
                "route",
                &Route {
                    len: self.route.len(),
                },
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

        let mut bindings = Vec::with_capacity(8);
        let mut branch = Vec::with_capacity(64);
        let mut nodes = Vec::with_capacity(1);
        let mut next = SmallVec::<[&[usize]; 2]>::new();

        if let Some(root) = self.tree.first() {
            nodes.push(Match::new(!segments.has_next(), None, &root.route));
            bindings.push(Binding::new(None, nodes));
            branch.extend_from_slice(&root.children);
        }

        while let Some((range, has_next)) = segments.next() {
            let mut nodes = Vec::with_capacity(4);

            for key in branch.drain(..) {
                let node = lookup!(&self.tree, key);

                match &node.pattern {
                    Pattern::Wildcard(param) => {
                        nodes.push(Match::new(true, Some(param), &node.route))
                    }
                    Pattern::Dynamic(param) => {
                        nodes.push(Match::new(!has_next, Some(param), &node.route));
                        next.push(&node.children);
                    }
                    Pattern::Static(value) => {
                        let [start, end] = range;

                        if value == &path[start..end] {
                            nodes.push(Match::new(!has_next, None, &node.route));
                            next.push(&node.children);
                        }
                    }
                }
            }

            bindings.push(Binding::new(Some(range), nodes));
            branch.extend(next.drain(..).flatten());
        }

        let mut wildcards = branch
            .drain(..)
            .filter_map(|key| self.match_trailing_wildcard(key))
            .peekable();

        if wildcards.peek().is_some() {
            bindings.push(Binding::new(None, wildcards.collect()));
        }

        bindings
    }

    fn match_trailing_wildcard(&self, key: usize) -> Option<Match<T>> {
        let node = self.tree.get(key)?;

        if let Pattern::Wildcard(param) = &node.pattern {
            Some(Match::new(true, Some(param), &node.route))
        } else {
            None
        }
    }
}

impl<T> Default for Router<T> {
    fn default() -> Self {
        Self {
            tree: vec![Node::new(Pattern::Static("".to_owned()))],
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
    use super::Router;

    static PATHS: [&str; 100] = [
        "/home",
        "/about",
        "/contact",
        "/login",
        "/signup",
        "/profile/:user_name",
        "/user/:user_id",
        "/settings",
        "/settings/account",
        "/settings/privacy",
        "/settings/security",
        "/posts",
        "/post/:post_id",
        "/post/:post_id/edit",
        "/post/:post_id/comments",
        "/post/:post_id/comments/:comment_id",
        "/post/:post_id/likes",
        "/post/:post_id/share",
        "/comments",
        "/comment/:comment_id",
        "/notifications",
        "/notifications/:notification_id",
        "/messages",
        "/message/:message_id",
        "/message/:message_id/reply",
        "/search",
        "/search/results",
        "/search/:query",
        "/admin",
        "/admin/users",
        "/admin/user/:user_id",
        "/admin/user/:user_id/edit",
        "/admin/posts",
        "/admin/post/:post_id",
        "/admin/post/:post_id/edit",
        "/admin/comments",
        "/admin/comment/:comment_id",
        "/admin/comment/:comment_id/edit",
        "/admin/categories",
        "/admin/category/:category_id",
        "/admin/category/:category_id/edit",
        "/admin/tags",
        "/admin/tag/:tag_id",
        "/admin/tag/:tag_id/edit",
        "/admin/settings",
        "/categories",
        "/category/:category_id",
        "/category/:category_id/posts",
        "/tags",
        "/tag/:tag_id",
        "/tag/:tag_id/posts",
        "/favorites",
        "/favorite/:item_id",
        "/friends",
        "/friend/:friend_id",
        "/groups",
        "/group/:group_id",
        "/group/:group_id/members",
        "/group/:group_id/posts",
        "/events",
        "/event/:event_id",
        "/event/:event_id/rsvp",
        "/event/:event_id/attendees",
        "/help",
        "/help/article/:article_id",
        "/terms",
        "/privacy",
        "/faq",
        "/sitemap",
        "/rss",
        "/api/:version/:resource",
        "/api/:version/:resource/:resource_id",
        "/api/:version/:resource/:resource_id/edit",
        "/api/:version/:resource/:resource_id/comments/:comment_id",
        "/api/:version/:resource/:resource_id/comments/:comment_id/edit",
        "/checkout",
        "/checkout/cart",
        "/checkout/payment",
        "/checkout/confirmation",
        "/dashboard",
        "/dashboard/overview",
        "/dashboard/stats",
        "/dashboard/reports",
        "/notifications/settings",
        "/notifications/settings/email",
        "/notifications/settings/include",
        "/inbox",
        "/inbox/:conversation_id",
        "/inbox/:conversation_id/messages",
        "/subscriptions",
        "/subscription/:subscription_id",
        "/subscription/:subscription_id/edit",
        "/billing",
        "/billing/history",
        "/billing/payment-methods",
        "/billing/invoice/:invoice_id",
        "/report/user/:user_id",
        "/report/post/:post_id",
        "/report/comment/:comment_id",
        "/invite",
    ];

    fn simple_prng(seed: u32) -> u32 {
        // Constants from Numerical Recipes
        let a: u64 = 1664525;
        let c: u64 = 1013904223;
        let m: u64 = u32::MAX as u64 + 1;

        let next = (a * seed as u64 + c) % m;
        next as u32 % 10_000 + 1
    }

    #[test]
    fn test_router_visit() {
        let mut router = Router::new();

        for pattern in PATHS {
            let _ = router.at(pattern).respond(());
        }

        for pattern in PATHS {
            let path = crate::path::Split::new(pattern)
                .map(|[start, end]| {
                    let segment = &pattern[start..end];
                    if segment.starts_with(':') {
                        simple_prng(12345).to_string()
                    } else {
                        segment.to_owned()
                    }
                })
                .fold(String::new(), |path, next| path + "/" + &next);

            router.visit(&path);
        }

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
