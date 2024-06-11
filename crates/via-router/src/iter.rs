use std::collections::VecDeque;

use crate::{
    node::{Node, Pattern},
    path::PathSegments,
    routes::RouteStore,
};

/// Represents either a partial or exact match for a given path segment.
#[derive(Clone, Copy)]
pub struct Match<'a, T> {
    /// Indicates whether or not the match is considered an exact match.
    /// If the match is exact, both the middleware and responders will be
    /// called during a request. Otherwise, only the middleware will be
    /// called.
    pub is_exact_match: bool,

    /// A key-value pair containing the start and end offset of the path
    /// segment that matches `self.pattern()`.
    pub path_segment: (usize, usize),

    /// The node that matches the value in the url path at `self.path_segment`.
    node: &'a Node<T>,
}

/// An iterator that yields all possible partial and exact matches for a url path.
pub struct Visit<'a, T> {
    matches: VecDeque<Match<'a, T>>,
}

struct Visitor<'a, 'b, T> {
    path: PathSegments<'b>,
    matches: &'b mut VecDeque<Match<'a, T>>,
    route_store: &'a RouteStore<T>,
}

impl<'a, T> Match<'a, T> {
    /// Returns a key-value pair where key is the name of the dynamic segment
    /// that was matched against and value is a key-value pair containing the
    /// start and end offset of the path segment in the url path. If the matched
    /// route does not have any dynamic segments, `None` will be returned.
    pub fn param(&self) -> Option<(&'static str, (usize, usize))> {
        if let Pattern::CatchAll(name) | Pattern::Dynamic(name) = self.pattern() {
            Some((name, self.path_segment))
        } else {
            None
        }
    }

    pub fn pattern(&self) -> Pattern {
        self.node.pattern
    }

    /// Returns a reference to the route that matches `self.value`.
    pub fn route(&self) -> Option<&'a T> {
        self.node.route.as_ref()
    }
}

impl<'a, T> Visit<'a, T> {
    /// Returns a new visitor to begin our search at the root `node` that match
    /// the provided `path`.
    pub(crate) fn new(store: &'a RouteStore<T>, node: &'a Node<T>, path: &str) -> Self {
        let mut matches = VecDeque::with_capacity(32);

        Visitor::visit(
            Visitor {
                path: PathSegments::new(path),
                matches: &mut matches,
                route_store: store,
            },
            node,
        );

        Visit { matches }
    }
}

impl<'a, T> Iterator for Visit<'a, T> {
    type Item = Match<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.matches.pop_front()
    }
}

impl<'a, 'b, T> Visitor<'a, 'b, T> {
    fn visit(mut self, root: &'a Node<T>) {
        // The root node is a special case that we always consider a match.
        self.matches.push_back(Match {
            is_exact_match: self.path.value == "/",
            path_segment: (0, 0),
            node: root,
        });

        // Begin the search for matches recursively starting with descendants of
        // the root node.
        self.match_at_depth(0, root);
    }

    fn match_at_depth(&mut self, depth: usize, node: &'a Node<T>) {
        if let Some(path_segment_range) = self.path.get(depth) {
            return self.match_segment_at_depth(path_segment_range, depth, node);
        }

        // If there is no path segment to match against, we attempt to find an
        // immediate descendant node with a CatchAll pattern. This is required
        // to support matching the "index" path of a descendant node with a
        // CatchAll pattern.
        for key in node.entries() {
            let next = &self.route_store[*key];

            // If the next node does not have a CatchAll pattern, we can skip
            // this node and continue to search for adjacent nodes with a
            // CatchAll pattern.
            if !matches!(next.pattern, Pattern::CatchAll(_)) {
                continue;
            }

            self.matches.push_back(Match {
                is_exact_match: true,
                path_segment: self.path.slice_from(depth),
                node: next,
            });
        }
    }

    /// Attempt to match the path segment located at `start` and `end` against the
    /// patterns in the current node. If a match is found, we will continue to match
    /// the path segment at the next depth against the patterns at the matching node.
    fn match_segment_at_depth(
        &mut self,
        (start, end): (usize, usize),
        depth: usize,
        node: &'a Node<T>,
    ) {
        let path_segment_value = &self.path.value[start..end];

        for key in node.entries() {
            let next = &self.route_store[*key];

            if path_segment_value != next.pattern {
                // The path segment does not match the pattern of the next node.
                // We can skip this node and continue to search for a match.
                continue;
            }

            if matches!(next.pattern, Pattern::CatchAll(_)) {
                // The next node has a `CatchAll` pattern and will be considered
                // an exact match. Due to the nature of `CatchAll` patterns, we
                // do not have to continue searching for descendants of this
                // node that match the remaining path segments.
                self.matches.push_back(Match {
                    is_exact_match: true,
                    // The end offset of `path_segment` should be the end offset
                    // of the last path segment in the url path.
                    path_segment: self.path.slice_from(depth),
                    node: next,
                });
            } else {
                self.matches.push_back(Match {
                    is_exact_match: self.path.is_last_segment(depth),
                    path_segment: (start, end),
                    node: next,
                });
                // Continue to match descendants of `next` against the path
                // segment at the next depth.
                self.match_at_depth(depth + 1, next);
            }
        }
    }
}
