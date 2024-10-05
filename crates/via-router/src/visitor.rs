use crate::path::{Param, Pattern, Span};
use crate::routes::RouteStore;
use crate::stack_vec::StackVec;

/// A matched node in the route tree.
///
/// Contains a reference to the route associated with the node and additional
/// metadata about the match.
///
#[derive(Debug)]
pub struct Found {
    /// True if there were no more segments to match against the children of the
    /// matched node. Otherwise, false.
    ///
    pub is_leaf: bool,

    /// A reference to the name of the dynamic parameter that matched the path
    /// segment.
    ///
    pub param: Option<Param>,

    /// An array containing the start and end index of the path segment that
    /// matched the node containing `route`.
    ///
    pub at: Span,
}

pub fn visit<T>(
    path: &str,
    store: &RouteStore<T>,
    segments: &StackVec<Span, 5>,
) -> Vec<(Option<usize>, Found)> {
    let mut results = Vec::new();
    let mut found = Found {
        // The root node's path segment range should produce to an empty str.
        at: Span::new(0, 0),
        param: None,
        // Assume we have segments is not empty.
        is_leaf: false,
    };
    let root = store.get(0);

    if let Some(range) = segments.get(0) {
        // Append the root match to the results vector.
        results.push((root.route, found));

        // Begin the search for matches recursively starting with descendants of
        // the root node.
        visit_node(&mut results, path, store, segments, range, 0, 0);
    } else {
        // Consider the root match a leaf since there are no segments to match
        // against.
        found.is_leaf = true;

        // Append the root match to the results vector.
        results.push((root.route, found));

        // Perform a shallow search for descendants of the root node that have a
        // `CatchAll` pattern.
        visit_index(&mut results, store, 0);
    }

    results
}

/// Recursively search for descendants of the node at `key` that have a
/// pattern that matches the path segment at `index`. If a match is found,
/// we'll add it to `results` and continue our search with the descendants
/// of matched node against the path segment at next index.
fn visit_node<T>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<(Option<usize>, Found)>,

    // The url path that we are attempting to match against the route tree.
    path: &str,

    // A reference to the route store that contains the route tree.
    store: &RouteStore<T>,

    segments: &StackVec<Span, 5>,

    // The start and end offset of the path segment at `index` in
    // `self.path_value`.
    range: &Span,

    // The index of the path segment in `self.segments` that we are matching
    // against the node at `key`.
    index: usize,

    // The key of the parent node that contains the descendants that we are
    // attempting to match against the path segment at `index`.
    key: usize,
) {
    // Get the value of the path segment at `range`. We'll eagerly borrow
    // and cache this slice from `path` to avoid having to build the ref
    // for each descendant with a static pattern.
    let segment = path.get(range.start..range.end).unwrap_or("");

    // Eagerly calculate and store the next index to avoid having to do so
    // for each descendant with a dynamic or static pattern.
    let next_index = index + 1;

    // Iterate over the keys of the descendants of the node at `key`.
    for next_key in store.get(key).entries().copied() {
        // Get the node at `next_key` from the route store.
        let descendant = store.get(next_key);
        let route = descendant.route;

        // Check if `descendant` has a pattern that matches `path_segment`.
        match &descendant.pattern {
            // The next node has a `Static` pattern that matches the value
            // of the path segment.
            Pattern::Static(param) if segment == param.as_str() => {
                let mut found = Found {
                    at: range.clone(),
                    param: None,
                    is_leaf: false,
                };

                if let Some(next_range) = segments.get(next_index) {
                    results.push((route, found));

                    // Continue searching for matches with the descendants of the
                    // current node against the next path segment.
                    visit_node(
                        results, path, store, segments, next_range, next_index, next_key,
                    );
                } else {
                    found.is_leaf = true;

                    results.push((route, found));

                    visit_index(results, store, next_key);
                }
            }

            // The next node has a `Dynamic` pattern. Therefore, we consider
            // it a match regardless of the value of the path segment.
            Pattern::Dynamic(param) => {
                let mut found = Found {
                    at: range.clone(),
                    param: Some(param.clone()),
                    is_leaf: false,
                };

                if let Some(next_range) = segments.get(next_index) {
                    results.push((route, found));

                    // Continue searching for matches with the descendants of the
                    // current node against the next path segment.
                    visit_node(
                        results, path, store, segments, next_range, next_index, next_key,
                    );
                } else {
                    found.is_leaf = true;
                    results.push((route, found));

                    visit_index(results, store, next_key);
                }
            }

            // The next node has a `CatchAll` pattern and will be considered
            // an exact match. Due to the nature of `CatchAll` patterns, we
            // do not have to continue searching for descendants of this
            // node that match the remaining path segments.
            Pattern::CatchAll(param) => {
                let found = Found {
                    // The end offset of `path_segment_range` should be the end
                    // offset of the last path segment in the url path since
                    // `CatchAll` patterns match the entire remainder of the
                    // url path from which they are matched.
                    at: Span::new(range.start, path.len()),
                    param: Some(param.clone()),
                    // `CatchAll` patterns are always considered a leaf node.
                    is_leaf: true,
                };

                results.push((route, found));
            }

            // We don't have to check and see if the pattern is `Pattern::Root`
            // since we already added our root node to the matches vector.
            _ => {}
        }
    }
}

/// Recursively search for matches in the route tree starting with the
/// descendants of the node at `key`. If there is not a path segment in
/// `self.segements` at `index` to match against the descendants of the
/// node at `key`, we'll instead perform a shallow search for descendants
/// with a `CatchAll` pattern.
fn visit_index<T>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<(Option<usize>, Found)>,

    // A reference to the route store that contains the route tree.
    store: &RouteStore<T>,

    // The key of the parent node that contains the descendants that we are
    // attempting to match against the path segment at `index`.
    key: usize,
) {
    // Perform a shallow search for descendants of the current node that
    // have a `CatchAll` pattern. This is required to support matching the
    // "index" path of a descendant node with a `CatchAll` pattern.
    for next_key in store.get(key).entries().copied() {
        // Get the node at `next_key` from the route store.
        let descendant = store.get(next_key);
        let route = descendant.route;

        // Check if `descendant` has a `CatchAll` pattern.
        if let Pattern::CatchAll(param) = &descendant.pattern {
            let found = Found {
                // Due to the fact we are looking for `CatchAll` patterns as
                // an immediate descendant of a node that we consider a match,
                // we can safely assume that the path segment range should
                // always produce an empty str.
                at: Span::new(0, 0),
                param: Some(param.clone()),
                // `CatchAll` patterns are always considered an exact match.
                is_leaf: true,
            };

            // Add the matching node to the vector of matches and continue to
            // search for adjacent nodes with a `CatchAll` pattern.
            results.push((route, found));
        }
    }
}
