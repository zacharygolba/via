#![allow(clippy::single_match)]

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::path::{Pattern, Segments};
use crate::routes::Node;
use crate::Param;

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
    pub exact: bool,

    /// The name of the dynamic parameter that matched the path segment.
    ///
    #[allow(clippy::type_complexity)]
    pub param: Option<(Param, Option<(usize, usize)>)>,

    /// The key of the route associated with the node that matched the path
    /// segment.
    ///
    pub route: Option<usize>,
}

impl Error for VisitError {}

impl Display for VisitError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
pub fn visit_node(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Result<Found, VisitError>>,

    // A reference to the route store that contains the route tree.
    nodes: &[Node],

    children: &[usize],

    segments: &Segments,

    // The start and end offset of the current path segment.
    (segment, at): (&str, &(usize, usize)),

    // The index of the next path segment in `self.segments`.
    current_index: usize,
) {
    // Cache the value of the next index to avoid arithmetic errors in a loop.
    let next_index = current_index + 1;

    // Cache an option containing the next path segment range to avoid slice access in a
    // loop.
    let next_segment = segments.get(next_index);

    // Search for descendant nodes that match `segment`.
    for key in children {
        #[rustfmt::skip]
        let (child, found) = match nodes.get(*key) {
            // The node has a static pattern. Attempt to match the pattern value against
            // the current path segment.
            Some(node @ Node { pattern: Pattern::Static(name), .. }) if name == segment => (
                node,
                Found {
                    exact: next_segment.is_none(),
                    param: None,
                    route: node.route,
                },
            ),

            // The node has a wildcard pattern. Consider it a match unconditionally and
            // use the length of the path as the end offset for the range.
            Some(node @ Node { pattern: Pattern::Wildcard(name), .. }) => (
                node,
                Found {
                    exact: true,
                    param: Some((name.clone(), Some((at.0, segments.path_len())))),
                    route: node.route,
                },
            ),

            // The node has a dynamic pattern. Consider it a match unconditionally.
            Some(node @ Node { pattern: Pattern::Dynamic(name), .. }) => (
                node,
                Found {
                    exact: next_segment.is_none(),
                    param: Some((name.clone(), Some(*at))),
                    route: node.route,
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

        match (&child.pattern, &child.children, next_segment) {
            // Wildcard patterns consume the remainder of the path. Continue matching
            // adjacent nodes.
            (Pattern::Wildcard(_), _, _) => {}

            // Perform a recursive search for descendants of `child` that match the next
            // path segment.
            (_, Some(children), Some(next_segment)) => {
                visit_node(results, nodes, children, segments, next_segment, next_index);
            }

            // Perform a shallow search for descendants of `child` with a wildcard pattern.
            (_, Some(children), None) => {
                visit_wildcard(results, nodes, children);
            }

            _ => {}
        }
    }
}

/// Accumulate child nodes with a wildcard pattern in results.
///
pub fn visit_wildcard(
    // A mutable reference to a vector that contains the matches that we found
    // so far.
    results: &mut Vec<Result<Found, VisitError>>,

    // A slice containing all of the nodes in the route tree.
    nodes: &[Node],

    children: &[usize],
) {
    for key in children {
        match nodes.get(*key) {
            #[rustfmt::skip]
            // The node has a wildcard pattern. Consider it a match unconditionally and
            // use the length of the path as the end offset for the range.
            Some(node @ Node { pattern: Pattern::Wildcard(name), .. }) => {
                results.push(Ok(Found {
                    exact: true,
                    param: Some((name.clone(), None)),
                    route: node.route,
                }));
            }

            // Continue searching for adjacent nodes with a wildcard pattern.
            Some(_) | None => {}
        }
    }
}
