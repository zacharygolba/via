#![allow(clippy::single_match)]

use std::error::Error;
use std::fmt::{self, Display};

use crate::path::Pattern;
use crate::routes::Node;

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
pub struct Found<'a> {
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
    pub param: Option<&'a str>,

    /// The range of the path segment that matched the node.
    ///
    pub at: Option<(usize, usize)>,
}

impl Error for VisitError {}

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

/// Recursively search for descendants of the node at `key` that have a
/// pattern that matches the path segment at `index`. If a match is found,
/// we'll add it to `results` and continue our search with the descendants
/// of matched node against the path segment at next index.
pub fn visit_node<'a>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Result<Found<'a>, VisitError>>,

    // A reference to the route store that contains the route tree.
    nodes: &'a [Node],

    // A reference to the most recently matched node.
    parent: &'a Node,

    // The url path that we are attempting to match against the route tree.
    path: &str,

    segments: &[(usize, usize)],

    // The start and end offset of the current path segment.
    range: &(usize, usize),

    // The index of the next path segment in `self.segments`.
    index: usize,
) {
    let (start, end) = *range;

    // Build a str with the value of the path segment at `index` from the range argument.
    let segment = &path[start..end];

    // Cache an option containing the next path segment range to avoid slice access in a
    // loop.
    let next = segments.get(index + 1);

    // Search for descendant nodes that match `segment`.
    for key in parent.entries().copied() {
        #[rustfmt::skip]
        let (child, found) = match nodes.get(key) {
            // The node has a static pattern. Attempt to match the pattern value against
            // the current path segment.
            Some(node @ Node { pattern: Pattern::Static(value), .. }) if value == segment => (
                node,
                Found {
                    is_leaf: next.is_none(),
                    param: None,
                    route: node.route,
                    at: Some((start, end)),
                },
            ),

            // The node has a wildcard pattern. Consider it a match unconditionally and
            // use the length of the path as the end offset for the range.
            Some(node @ Node { pattern: Pattern::Wildcard(name), .. }) => (
                node,
                Found {
                    is_leaf: true,
                    param: Some(name),
                    route: node.route,
                    at: Some((start, path.len())),
                },
            ),

            // The node has a dynamic pattern. Consider it a match unconditionally.
            Some(node @ Node { pattern: Pattern::Dynamic(name), .. }) => (
                node,
                Found {
                    is_leaf: next.is_none(),
                    param: Some(name),
                    route: node.route,
                    at: Some((start, end)),
                },
            ),

            // The node does not match the current path segment.
            Some(_) => {
                continue;
            }

            // The node at `key` does not exist.
            None => {
                // Append an error result to the matches vector.
                results.push(Err(VisitError::NodeNotFound));
                // Stop searching for nodes because memory is likely corrupt.
                break;
            }
        };

        // Append the match to the results vector.
        results.push(Ok(found));

        match (next, &child.pattern) {
            // Wildcard patterns consume the remainder of the path. Continue matching
            // adjacent nodes.
            (_, Pattern::Wildcard(_)) => {}

            // Perform a recursive search for descendants of `child` that match the next
            // path segment.
            (Some(range), _) => {
                visit_node(results, nodes, child, path, segments, range, index + 1);
            }

            // Perform a shallow search for descendants of `child` with a wildcard pattern.
            (None, _) => {
                visit_wildcard(results, nodes, child);
            }
        }
    }
}

/// Accumulate child nodes with a wildcard pattern in results.
///
pub fn visit_wildcard<'a>(
    // A mutable reference to a vector that contains the matches that we found
    // so far.
    results: &mut Vec<Result<Found<'a>, VisitError>>,

    // A slice containing all of the nodes in the route tree.
    nodes: &'a [Node],

    // A reference to the most recently matched node.
    parent: &'a Node,
) {
    for key in parent.entries().copied() {
        match nodes.get(key) {
            #[rustfmt::skip]
            // The node has a wildcard pattern. Consider it a match unconditionally and
            // use the length of the path as the end offset for the range.
            Some(node @ Node { pattern: Pattern::Wildcard(name), .. }) => {
                results.push(Ok(Found {
                    is_leaf: true,
                    param: Some(name),
                    route: node.route,
                    at: None,
                }));
            }

            // Continue searching for adjacent nodes with a wildcard pattern.
            Some(_) | None => {}
        }
    }
}
