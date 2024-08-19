use crate::path::{self, Pattern};
use crate::routes::RouteStore;

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

pub fn visit<'a, T>(
    matched_routes: &mut Vec<Match<'a, T>>,
    route_store: &'a RouteStore<T>,
    segments: &[(usize, usize)],
    path: &str,
) {
    // The root node is a special case that we always consider a match.
    matched_routes.push(Match {
        // If there are no path segments to match against, we consider the root
        // node to be an exact match.
        exact: segments.is_empty(),
        // The root node cannot have parameters.
        param: None,
        // The root node's path segment range should produce to an empty str.
        range: (0, 0),
        route: route_store.route_at_node(0),
    });

    // Begin the search for matches recursively starting with descendants of
    // the root node.
    visit_node(matched_routes, route_store, &segments, path, 0, 0);
}

/// Recursively search for descendants of the current node that have a pattern
/// that matches the path segment at `range`. If a match is found, we will add
/// it to our vector of matches and continue to search for matching nodes at
/// the next depth in the route tree.
fn visit_matching_entries<'a, T>(
    matched_routes: &mut Vec<Match<'a, T>>,
    route_store: &'a RouteStore<T>,
    segments: &[(usize, usize)],
    range: &(usize, usize),
    path: &str,
    index: usize,
    key: usize,
) {
    let next_index = index + 1;
    let segment = &path[range.0..range.1];
    let exact = next_index == segments.len();

    for entry in route_store.node(key).entries() {
        match route_store.node(*entry).pattern {
            Pattern::CatchAll(param) => {
                // The next node has a `CatchAll` pattern and will be considered
                // an exact match. Due to the nature of `CatchAll` patterns, we
                // do not have to continue searching for descendants of this
                // node that match the remaining path segments.
                matched_routes.push(Match {
                    // `CatchAll` patterns are always considered an exact match.
                    exact: true,
                    // The end offset of `path_segment_range` should be the end
                    // offset of the last path segment in the url path since
                    // `CatchAll` patterns match the entire remainder of the
                    // url path from which they are matched.
                    range: path::slice_segments_from(segments, index),
                    param: Some(param),
                    route: route_store.route_at_node(*entry),
                });
            }
            Pattern::Dynamic(param) => {
                // The next node has a `Dynamic` pattern. Therefore, we consider
                // it a match regardless of the value of the path segment.
                matched_routes.push(Match {
                    exact,
                    range: *range,
                    param: Some(param),
                    route: route_store.route_at_node(*entry),
                });

                visit_node(
                    matched_routes,
                    route_store,
                    segments,
                    path,
                    next_index,
                    *entry,
                );
            }
            Pattern::Static(value) if value == segment => {
                // The next node has a `Static` pattern that matches the value
                // of the path segment.
                matched_routes.push(Match {
                    exact,
                    range: *range,
                    param: None,
                    route: route_store.route_at_node(*entry),
                });

                visit_node(
                    matched_routes,
                    route_store,
                    segments,
                    path,
                    next_index,
                    *entry,
                );
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
fn visit_node<'a, T>(
    matched_routes: &mut Vec<Match<'a, T>>,
    route_store: &'a RouteStore<T>,
    segments: &[(usize, usize)],
    path: &str,
    index: usize,
    key: usize,
) {
    if let Some(range) = segments.get(index) {
        return visit_matching_entries(
            matched_routes,
            route_store,
            segments,
            range,
            path,
            index,
            key,
        );
    }

    // Perform a shallow search for descendants of the current node that have a
    // `CatchAll` pattern. This is required to support matching the "index" path
    // of a descendant node with a `CatchAll` pattern.
    for entry in route_store.node(key).entries() {
        // If the node at `entry` has a `CatchAll` pattern, we consider it a match.
        if let Pattern::CatchAll(param) = route_store.node(*entry).pattern {
            // Add the matching node to the vector of matches and continue to
            // search for adjacent nodes with a `CatchAll` pattern.
            matched_routes.push(Match {
                route: route_store.route_at_node(*entry),
                // `CatchAll` patterns are always considered an exact match.
                exact: true,
                // Include the name of the dynamic segment even if the value
                // of the path segment is empty for API consistency.
                param: Some(param),
                // Due to the fact we are looking for `CatchAll` patterns as an
                // immediate descendant of a node that we consider a match, we
                // can safely assume that the path segment range should always
                // produce an empty str.
                range: (0, 0),
            });
        }
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

impl<'a, T> Match<'a, Vec<T>> {
    /// Returns an iterator that yields a reference to each item in the matched
    /// route.
    pub fn iter(&self) -> impl Iterator<Item = &'a T> {
        self.route.map(|route| route.iter()).into_iter().flatten()
    }
}
