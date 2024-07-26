use crate::{
    path::{self, PathSegments, Pattern},
    routes::{Node, RouteStore},
};

/// Represents either a partial or exact match for a given path segment.
pub struct Match<'a, T> {
    /// Indicates whether or not the match is considered an exact match.
    /// If the match is exact, both the middleware and responders will be
    /// called during a request. Otherwise, only the middleware will be
    /// called.
    pub is_exact_match: bool,

    /// A tuple that contains the start and end offset of the path segment that
    /// matches `self.route()`.
    pub path_segment_range: (usize, usize),

    /// The node that matches the value in the url path at `self.path_segment`.
    node: &'a Node<T>,
}

pub(crate) struct Visitor<'a, 'b, T> {
    path: PathSegments<'b>,
    matches: Vec<Match<'a, T>>,
    routes: &'a RouteStore<T>,
}

impl<'a, T> Match<'a, T> {
    /// Returns a key-value pair where key is the name of the dynamic segment
    /// that was matched against and value is a key-value pair containing the
    /// start and end offset of the path segment in the url path. If the matched
    /// route does not have any dynamic segments, `None` will be returned.
    pub fn param(&self) -> Option<(&'static str, (usize, usize))> {
        if let Pattern::CatchAll(name) | Pattern::Dynamic(name) = self.node.pattern {
            Some((name, self.path_segment_range))
        } else {
            None
        }
    }

    /// Returns a reference to the route that matches `self.value`.
    pub fn route(&self) -> Option<&'a T> {
        self.node.route.as_deref()
    }
}

impl<'a, 'b, T> Visitor<'a, 'b, T> {
    pub(crate) fn new(routes: &'a RouteStore<T>, path: &'b str) -> Self {
        let matches = Vec::with_capacity(32);

        Self {
            routes,
            matches,
            path: path::segments(path),
        }
    }

    pub(crate) fn visit(mut self, root: &'a Node<T>) -> Vec<Match<'a, T>> {
        // The root node is a special case that we always consider a match.
        self.matches.push(Match {
            is_exact_match: self.path.get(0).is_none(),
            path_segment_range: (0, 0),
            node: root,
        });

        // Begin the search for matches recursively starting with descendants of
        // the root node.
        self.match_at_depth(0, root);
        self.matches
    }

    fn match_at_depth(&mut self, depth: usize, node: &'a Node<T>) {
        if let Some(path_segment_range) = self.path.get(depth) {
            self.match_segment_at_depth(*path_segment_range, depth, node);
            return;
        }

        // If there is no path segment to match against, we attempt to find an
        // immediate descendant node with a CatchAll pattern. This is required
        // to support matching the "index" path of a descendant node with a
        // CatchAll pattern.
        for index in node.entries() {
            let next = self.routes.get(*index);

            // If the next node does not have a CatchAll pattern, we can skip
            // this node and continue to search for adjacent nodes with a
            // CatchAll pattern.
            if let Pattern::CatchAll(_) = next.pattern {
                self.matches.push(Match {
                    is_exact_match: true,
                    path_segment_range: self.path.slice_from(depth),
                    node: next,
                });
            }
        }
    }

    /// Attempt to match the path segment located at `start` and `end` against the
    /// patterns in the current node. If a match is found, we will continue to match
    /// the path segment at the next depth against the patterns at the matching node.
    fn match_segment_at_depth(
        &mut self,
        path_segment_range: (usize, usize),
        depth: usize,
        node: &'a Node<T>,
    ) {
        let (start, end) = path_segment_range;
        let is_exact_match = self.path.get(depth + 1).is_none();
        let path_segment_value = &self.path.value[start..end];

        for index in node.entries() {
            let next = self.routes.get(*index);

            match next.pattern {
                Pattern::CatchAll(_) => {
                    // The next node has a `CatchAll` pattern and will be considered
                    // an exact match. Due to the nature of `CatchAll` patterns, we
                    // do not have to continue searching for descendants of this
                    // node that match the remaining path segments.
                    self.matches.push(Match {
                        // The end offset of `path_segment` should be the end offset
                        // of the last path segment in the url path.
                        path_segment_range: self.path.slice_from(depth),
                        is_exact_match: true,
                        node: next,
                    });
                }
                Pattern::Dynamic(_) => {
                    self.matches.push(Match {
                        node: next,
                        is_exact_match,
                        path_segment_range,
                    });
                    self.match_at_depth(depth + 1, next);
                }
                Pattern::Static(value) if value == path_segment_value => {
                    self.matches.push(Match {
                        node: next,
                        is_exact_match,
                        path_segment_range,
                    });
                    self.match_at_depth(depth + 1, next);
                }
                _ => {
                    // We don't have to check and see if the pattern is `Pattern::Root`
                    // since we already added our root node to the matches vector.
                }
            }
        }
    }
}
