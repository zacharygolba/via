use std::error::Error;
use std::fmt::{self, Display};

use crate::path::{Pattern, Span};
use crate::routes::Node;

macro_rules! expect_entry {
    ($results:expr, $nodes:expr, $key:expr) => {
        match $nodes.get($key) {
            Some(descendant) => descendant,
            None => {
                $results.push(Err(crate::VisitError::NodeNotFound));
                continue;
            }
        }
    };
}

#[derive(Clone, Debug)]
pub enum VisitError {
    /// The route tree is missing a node that is referenced by another node.
    //
    // This is an unlikely error that could indicate that the memory where the
    // route tree is stored has been corrupted.
    //
    NodeNotFound,

    /// The route tree is missing the root node.
    //
    // This is a *very* unlikely error that could indicate that the memory where
    // the route tree is stored has been corrupted.
    //
    RootNotFound,
}

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
    pub param: Option<String>,

    /// The range of the path segment that matched the node.
    ///
    pub at: Option<Span>,
}

pub fn visit(
    results: &mut Vec<Result<Found, VisitError>>,
    nodes: &[Node],
    segments: &[Span],
    path: &str,
) {
    let root = match nodes.first() {
        Some(node) => node,
        None => {
            results.push(Err(VisitError::NodeNotFound));
            return;
        }
    };

    match segments.first() {
        Some(range) => {
            // Append the root match to the results vector.
            results.push(Ok(Found::new(root.route, None, None)));

            // Begin the search for matches recursively starting with descendants of
            // the root node.
            visit_node(results, nodes, root, path, segments, range, 0);
        }
        None => {
            // Append the root match to the results vector.
            results.push(Ok(Found::leaf(root.route, None, None)));

            // Perform a shallow search for descendants of the root node that have a
            // `CatchAll` pattern.
            visit_index(results, nodes, root);
        }
    }
}

impl Display for VisitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound => {
                write!(f, "a node was visited that contains an invalid reference")
            }
            Self::RootNotFound => {
                write!(f, "the route tree is missing the root node")
            }
        }
    }
}

impl Error for VisitError {}

impl Found {
    #[inline]
    fn new(route: Option<usize>, param: Option<String>, at: Option<Span>) -> Self {
        Self {
            is_leaf: false,
            route,
            param,
            at,
        }
    }

    #[inline]
    fn leaf(route: Option<usize>, param: Option<String>, at: Option<Span>) -> Self {
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
    results: &mut Vec<Result<Found, VisitError>>,

    // A reference to the route store that contains the route tree.
    nodes: &[Node],

    // A reference to the most recently matched node.
    node: &Node,

    // The url path that we are attempting to match against the route tree.
    path: &str,

    segments: &[Span],

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
    let segment = &path[range.start()..range.end()];

    // Search for descendant nodes that match `segment`.
    for key in node.entries().copied() {
        let entry = expect_entry!(results, nodes, key);

        // Check if `descendant` has a pattern that matches `path_segment`.
        match &entry.pattern {
            // The next node has a `Static` pattern that matches the value
            // of the path segment.
            Pattern::Static(param) if param == segment => {
                // Calculate the index of the next path segment.
                let next_index = index + 1;
                let at = Some(range.clone());

                match segments.get(next_index) {
                    Some(next_range) => {
                        // Append the match to the results vector.
                        results.push(Ok(Found::new(entry.route, None, at)));

                        // Continue searching for descendants of the current node
                        // that match the the next path segment.
                        visit_node(
                            results, nodes, entry, path, segments, next_range, next_index,
                        );
                    }
                    None => {
                        // Append the match to the results vector.
                        results.push(Ok(Found::leaf(entry.route, None, at)));

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
                let next_index = index + 1;
                let param = Some(param.to_owned());
                let at = Some(range.clone());

                match segments.get(next_index) {
                    Some(next_range) => {
                        // Append the match to the results vector.
                        results.push(Ok(Found::new(entry.route, param, at)));

                        // Continue searching for descendants of the current node
                        // that match the the next path segment.
                        visit_node(
                            results, nodes, entry, path, segments, next_range, next_index,
                        );
                    }
                    None => {
                        // Append the match to the results vector.
                        results.push(Ok(Found::leaf(entry.route, param, at)));

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
                results.push(Ok(Found::leaf(
                    entry.route,
                    Some(param.to_owned()),
                    Some(Span::new(range.start(), path.len())),
                )));
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
    results: &mut Vec<Result<Found, VisitError>>,

    // A reference to the route store that contains the route tree.
    nodes: &[Node],

    // A reference to the most recently matched node.
    node: &Node,
) {
    // Perform a shallow search for descendants of the current node that
    // have a `CatchAll` pattern. This is required to support matching the
    // "index" path of a descendant node with a `CatchAll` pattern.
    for key in node.entries().copied() {
        let entry = expect_entry!(results, nodes, key);

        // Check if `descendant` has a `CatchAll` pattern.
        if let Pattern::Wildcard(param) = &entry.pattern {
            // Append the match as a leaf to the results vector.
            results.push(Ok(Found::leaf(entry.route, Some(param.to_owned()), None)));
        }
    }
}
