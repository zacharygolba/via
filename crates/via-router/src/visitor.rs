use crate::{
    path::{PathSegments, Pattern},
    routes::{Node, RouteStore},
};

/// Represents either a partial or exact match for a given path segment.
pub struct Match<'a, T> {
    /// Indicates whether or not the match is considered an exact match.
    /// If the match is exact, both the middleware and responders will be
    /// called during a request. Otherwise, only the middleware will be
    /// called.
    pub exact: bool,

    /// A tuple that contains the start and end offset of the path segment that
    /// matches `self.route()`.
    pub range: (usize, usize),

    /// The route that matches the path segement at `self.range`.
    pub route: Option<&'a T>,

    /// The name of the dynamic segment that was matched against.
    param: Option<&'static str>,
}

struct Visitor<'a, 'b, T> {
    matched_routes: &'b mut Vec<Match<'a, T>>,
    path_segments: &'b PathSegments<'b>,
    route_store: &'a RouteStore<T>,
}

struct Visit<'a> {
    /// The index of the path segment that matches `self.node`.
    index: usize,

    /// The node that matches the path segment at `self.index`.
    node: &'a Node,
}

pub fn visit<'a, 'b, T>(
    matched_routes: &'b mut Vec<Match<'a, T>>,
    path_segments: &'b PathSegments<'b>,
    route_store: &'a RouteStore<T>,
    node: &'a Node,
) {
    let mut visitor = Visitor {
        matched_routes,
        path_segments,
        route_store,
    };

    // The root node is a special case that we always consider a match.
    visitor.push(
        // If there are no path segments to match against, we consider the root
        // node to be an exact match.
        path_segments.is_empty(),
        // The root node's path segment range should produce to an empty str.
        (0, 0),
        None,
        0,
    );

    // Begin the search for matches recursively starting with descendants of
    // the root node.
    visit_node(&mut visitor, Visit { index: 0, node });
}

/// Perform a shallow search for descendants of the current node that have a
/// `CatchAll` pattern. This is required to support matching the "index" path
/// of a descendant node with a `CatchAll` pattern.
fn visit_catch_all_entries<T>(visitor: &mut Visitor<'_, '_, T>, visit: Visit) {
    for index in visit.node.entries().copied() {
        let node = visitor.route_store.node(index);

        if let Pattern::CatchAll(param) = node.pattern {
            // Add the matching node to the vector of matches and continue to
            // search for adjacent nodes with a `CatchAll` pattern.
            visitor.push(
                // `CatchAll` patterns are always considered an exact match.
                true,
                // Due to the fact we are looking for `CatchAll` patterns as an
                // immediate descendant of a node that we consider a match, we
                // can safely assume that the path segment range should always
                // produce an empty str.
                (0, 0),
                // Include the name of the dynamic segment even if the value
                // of the path segment is empty.
                Some(param),
                index,
            );
        }
    }
}

/// Recursively search for descendants of the current node that have a pattern
/// that matches the path segment at `range`. If a match is found, we will add
/// it to our vector of matches and continue to search for matching nodes at
/// the next depth in the route tree.
fn visit_matching_entries<'a, T>(
    visitor: &mut Visitor<'a, '_, T>,
    visit: Visit<'a>,
    range: (usize, usize),
) {
    let path_segment = &visitor.path_segments.value[range.0..range.1];
    let next_index = visit.index + 1;
    let exact = next_index == visitor.path_segments.len();

    for index in visit.node.entries().copied() {
        let node = visitor.route_store.node(index);

        match node.pattern {
            Pattern::CatchAll(param) => {
                // The next node has a `CatchAll` pattern and will be considered
                // an exact match. Due to the nature of `CatchAll` patterns, we
                // do not have to continue searching for descendants of this
                // node that match the remaining path segments.
                visitor.push(
                    // `CatchAll` patterns are always considered an exact match.
                    true,
                    // The end offset of `path_segment_range` should be the end
                    // offset of the last path segment in the url path since
                    // `CatchAll` patterns match the entire remainder of the
                    // url path from which they are matched.
                    visitor.path_segments.slice_from(visit.index),
                    Some(param),
                    index,
                );
            }
            Pattern::Dynamic(param) => {
                // The next node has a `Dynamic` pattern. Therefore, we consider
                // it a match regardless of the value of the path segment.
                visitor.push(exact, range, Some(param), index);
                visit_node(visitor, visit.next(node));
            }
            Pattern::Static(value) if value == path_segment => {
                // The next node has a `Static` pattern that matches the value
                // of the path segment.
                visitor.push(exact, range, None, index);
                visit_node(visitor, visit.next(node));
            }
            _ => {
                // We don't have to check and see if the pattern is `Pattern::Root`
                // since we already added our root node to the matches vector.
            }
        }
    }
}

/// Recursively search for matches in the route tree starting at the current node.
/// If there is no path segment to match against, we will attempt to find immediate
/// descendants of the current node with a `CatchAll` pattern.
fn visit_node<'a, T>(visitor: &mut Visitor<'a, '_, T>, visit: Visit<'a>) {
    if let Some(range) = visitor.path_segments.get(visit.index) {
        visit_matching_entries(visitor, visit, *range);
    } else {
        visit_catch_all_entries(visitor, visit);
    }
}

impl<'a, T> Match<'a, T> {
    /// Returns a key-value pair where key is the name of the dynamic segment
    /// that was matched against and value is a key-value pair containing the
    /// start and end offset of the path segment in the url path. If the matched
    /// route does not have any dynamic segments, `None` will be returned.
    pub fn param(&self) -> Option<(&'static str, (usize, usize))> {
        self.param.zip(Some(self.range))
    }
}

impl<'a, 'b, T> Visitor<'a, 'b, T> {
    fn push(
        &mut self,
        // Indicates whether or not the match is considered an exact match.
        exact: bool,
        // The start and end offset of the path segment that matches the node.
        range: (usize, usize),
        // The name of the dynamic segment that was matched against if the
        // matched node has a `CatchAll` or `Dynamic` pattern.
        param: Option<&'static str>,
        // The index of the node that matches the path segment at `range`.
        index: usize,
    ) {
        self.matched_routes.push(Match {
            route: self.route_store.route_at_node(index),
            exact,
            param,
            range,
        });
    }
}

impl<'a> Visit<'a> {
    fn next(&self, node: &'a Node) -> Self {
        Self {
            index: self.index + 1,
            node,
        }
    }
}
