use crate::path::{Param, Pattern, Span};
use crate::routes::Node;
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

    /// The key of the route associated with the node that matched the path
    /// segment.
    ///
    pub route: Option<usize>,

    /// The name of the dynamic parameter that matched the path segment.
    ///
    pub param: Option<Param>,

    /// The range of the path segment that matched the node.
    ///
    pub at: Span,
}

pub fn visit(path: &str, nodes: &[Node], segments: &StackVec<Span, 5>) -> Vec<Found> {
    let mut results = Vec::new();
    let root = match nodes.get(0) {
        Some(node) => node,
        None => {
            // This is an edge-case that can be caused by corrupt memory or a bug
            // in the router. We should log the error and not match any routes.
            // Placeholder for tracing...
            return results;
        }
    };

    match segments.get(0) {
        Some(range) => {
            // Append the root match to the results vector.
            results.push(Found::new(root.route, None, Span::new(0, 0)));

            // Begin the search for matches recursively starting with descendants of
            // the root node.
            visit_node(&mut results, nodes, root, path, segments, range, 0);
        }
        None => {
            // Append the root match to the results vector.
            results.push(Found::leaf(root.route, None, Span::new(0, 0)));

            // Perform a shallow search for descendants of the root node that have a
            // `CatchAll` pattern.
            visit_index(&mut results, nodes, root);
        }
    }

    results
}

impl Found {
    fn new(route: Option<usize>, param: Option<Param>, at: Span) -> Self {
        Self {
            is_leaf: false,
            route,
            param,
            at,
        }
    }

    fn leaf(route: Option<usize>, param: Option<Param>, at: Span) -> Self {
        Self {
            is_leaf: true,
            route,
            param,
            at,
        }
    }
}

/// Recursively search for descendants of the node at `key` that have a
/// pattern that matches the path segment at `index`. If a match is found,
/// we'll add it to `results` and continue our search with the descendants
/// of matched node against the path segment at next index.
fn visit_node(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Found>,

    // A reference to the route store that contains the route tree.
    nodes: &[Node],

    // A reference to the most recently matched node.
    node: &Node,

    // The url path that we are attempting to match against the route tree.
    path: &str,

    segments: &StackVec<Span, 5>,

    // The start and end offset of the path segment at `index` in
    // `self.path_value`.
    range: &Span,

    // The index of the path segment in `self.segments` that we are matching
    // against the node at `key`.
    index: usize,
) {
    // Get the value of the path segment at `range`. We'll eagerly borrow
    // and cache this slice from `path` to avoid having to build the ref
    // for each descendant with a static pattern.
    let segment = match path.get(range.start()..range.end()) {
        Some(slice) => slice,
        None => {
            // Placeholder for tracing...
            return;
        }
    };

    // Search for descendant nodes that match `segment`.
    for key in node.entries() {
        let entry = match nodes.get(*key) {
            Some(entry) => entry,
            None => {
                // Placeholder for tracing...
                continue;
            }
        };

        // Check if `descendant` has a pattern that matches `path_segment`.
        match &entry.pattern {
            // The next node has a `Static` pattern that matches the value
            // of the path segment.
            Pattern::Static(param) if segment == param.as_str() => {
                // Calculate the index of the next path segment.
                let index = index + 1;
                let at = range.clone();

                match segments.get(index) {
                    Some(range) => {
                        // Append the match to the results vector.
                        results.push(Found::new(entry.route, None, at));

                        // Continue searching for descendants of the current node
                        // that match the the next path segment.
                        visit_node(results, nodes, entry, path, segments, range, index);
                    }
                    None => {
                        // Append the match to the results vector.
                        results.push(Found::leaf(entry.route, None, at));

                        // Perform a shallow search for descendants of the
                        // current node that have a `CatchAll` pattern.
                        visit_index(results, nodes, entry);
                    }
                }
            }

            // The next node has a `Dynamic` pattern. Therefore, we consider
            // it a match regardless of the value of the path segment.
            Pattern::Dynamic(param) => {
                // Calculate the index of the next path segment.
                let index = index + 1;
                let at = range.clone();

                match segments.get(index) {
                    Some(range) => {
                        // Append the match to the results vector.
                        results.push(Found::new(entry.route, Some(param.clone()), at));

                        // Continue searching for descendants of the current node
                        // that match the the next path segment.
                        visit_node(results, nodes, entry, path, segments, range, index);
                    }
                    None => {
                        // Append the match to the results vector.
                        results.push(Found::leaf(entry.route, Some(param.clone()), at));

                        // Perform a shallow search for descendants of the
                        // current node that have a `CatchAll` pattern.
                        visit_index(results, nodes, entry);
                    }
                }
            }

            // The next node has a `CatchAll` pattern and will be considered
            // an exact match. Due to the nature of `CatchAll` patterns, we
            // do not have to continue searching for descendants of this
            // node that match the remaining path segments.
            Pattern::Wildcard(param) => {
                results.push(Found::leaf(
                    entry.route,
                    Some(param.clone()),
                    Span::new(range.start(), path.len()),
                ));
            }

            // We don't have to check and see if the pattern is `Pattern::Root`
            // since we already added our root node to the matches vector.
            _ => {}
        }
    }
}

/// Perform a shallow search for descendants of the `node` that have a `CatchAll`
/// pattern.
///
fn visit_index(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Found>,

    // A reference to the route store that contains the route tree.
    nodes: &[Node],

    // A reference to the most recently matched node.
    node: &Node,
) {
    // Perform a shallow search for descendants of the current node that
    // have a `CatchAll` pattern. This is required to support matching the
    // "index" path of a descendant node with a `CatchAll` pattern.
    for key in node.entries() {
        let entry = match nodes.get(*key) {
            Some(entry) => entry,
            None => {
                // Placeholder for tracing...
                continue;
            }
        };

        // Check if `descendant` has a `CatchAll` pattern.
        if let Pattern::Wildcard(param) = &entry.pattern {
            // Append the match as a leaf to the results vector.
            results.push(Found::leaf(
                entry.route,
                Some(param.clone()),
                Span::new(0, 0),
            ));
        }
    }
}
