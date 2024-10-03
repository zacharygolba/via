use crate::path::{Param, Pattern};
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

    /// A reference to the route referenced by the node that matched the path
    /// segment.
    ///
    pub key: Option<usize>,

    /// An array containing the start and end index of the path segment that
    /// matched the node containing `route`.
    ///
    pub start: usize,

    pub end: usize,
}

pub fn visit<'a, T>(
    results: &mut Vec<Found>,
    path: &str,
    store: &'a RouteStore<T>,
    segments: &StackVec<(usize, usize), 5>,
) {
    let root = store.get(0);

    // The root node is a special case that we always consider a match.
    results.push(Found {
        // If there are no path segments to match against, we consider the
        // root node to be an exact match.
        is_leaf: segments.is_empty(),
        // The root node's key is always `0`.
        param: None,
        key: root.route,
        // The root node's path segment range should produce to an empty str.
        start: 0,
        end: 0,
    });

    // Begin the search for matches recursively starting with descendants of
    // the root node.
    visit_node(results, path, store, segments, 0, 0)
}

/// Recursively search for descendants of the node at `key` that have a
/// pattern that matches the path segment at `index`. If a match is found,
/// we'll add it to `results` and continue our search with the descendants
/// of matched node against the path segment at next index.
fn visit_node<'a, T>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Found>,
    path: &str,
    store: &'a RouteStore<T>,
    segments: &StackVec<(usize, usize), 5>,
    // The index of the path segment in `self.segments` that we are matching
    // against the node at `key`.
    index: usize,
    // The key of the parent node that contains the descendants that we are
    // attempting to match against the path segment at `index`.
    key: usize,
) {
    let (start, end) = match segments.get(index) {
        Some(array) => array,
        None => {
            visit_index(results, store, key);
            return;
        }
    };

    // Get the value of the path segment at `index`. We'll eagerly borrow
    // and cache this slice from `self.path_value` to avoid having to build
    // the reference for each descendant with a `Static` pattern.
    let segment = path.get(start..end).unwrap_or("");

    // Eagerly calculate and store the next index to avoid having to do so
    // for each descendant with a `Dynamic` or `Static` pattern.
    let next_index = index + 1;

    // Use the value of `next_index` to determine if we are working with the
    // last path segment in `self.segments`. If so, we'll consider any
    // matching descendant to be a leaf node. We perform this check eagerly
    // to avoid having to do so for each descendant with a `Dynamic` or
    // `Static` pattern.
    let is_leaf = next_index == segments.len();

    // Iterate over the keys of the descendants of the node at `key`.
    for next_key in store.get(key).entries().copied() {
        // Get the node at `next_key` from the route store.
        let descendant = store.get(next_key);
        let param = descendant.param();

        // Check if `descendant` has a pattern that matches `segment`.
        match &descendant.pattern {
            Pattern::Static(value) if value == segment => {
                // The next node has a `Static` pattern that matches the value
                // of the path segment.
                results.push(Found {
                    is_leaf,
                    start,
                    end,
                    param,
                    key: descendant.route,
                });

                visit_node(results, path, store, segments, next_index, next_key);
            }
            Pattern::Dynamic(_) => {
                // The next node has a `Dynamic` pattern. Therefore, we consider
                // it a match regardless of the value of the path segment.
                results.push(Found {
                    is_leaf,
                    start,
                    end,
                    param,
                    key: descendant.route,
                });

                visit_node(results, path, store, segments, next_index, next_key);
            }
            Pattern::CatchAll(_) => {
                // The next node has a `CatchAll` pattern and will be considered
                // an exact match. Due to the nature of `CatchAll` patterns, we
                // do not have to continue searching for descendants of this
                // node that match the remaining path segments.
                results.push(Found {
                    start,
                    // `CatchAll` patterns are always considered a leaf node.
                    is_leaf: true,
                    param,
                    key: descendant.route,
                    // The end offset of `path_segment_range` should be the end
                    // offset of the last path segment in the url path since
                    // `CatchAll` patterns match the entire remainder of the
                    // url path from which they are matched.
                    end: path.len(),
                });
            }
            _ => {
                // We don't have to check and see if the pattern is `Pattern::Root`
                // since we already added our root node to the matches vector.
            }
        }
    }
}

/// Recursively search for matches in the route tree starting with the
/// descendants of the node at `key`. If there is not a path segment in
/// `self.segements` at `index` to match against the descendants of the
/// node at `key`, we'll instead perform a shallow search for descendants
/// with a `CatchAll` pattern.
fn visit_index<'a, T>(results: &mut Vec<Found>, store: &'a RouteStore<T>, key: usize) {
    // Perform a shallow search for descendants of the current node that
    // have a `CatchAll` pattern. This is required to support matching the
    // "index" path of a descendant node with a `CatchAll` pattern.
    for descendant in store.get(key).entries().map(|k| store.get(*k)) {
        if let Pattern::CatchAll(name) = &descendant.pattern {
            // Add the matching node to the vector of matches and continue to
            // search for adjacent nodes with a `CatchAll` pattern.
            results.push(Found {
                // `CatchAll` patterns are always considered an exact match.
                is_leaf: true,
                param: Some(name.clone()),
                key: descendant.route,
                // Due to the fact we are looking for `CatchAll` patterns as
                // an immediate descendant of a node that we consider a match,
                // we can safely assume that the path segment range should
                // always produce an empty str.
                start: 0,
                end: 0,
            });
        }
    }
}
