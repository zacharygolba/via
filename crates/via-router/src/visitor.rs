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

pub fn visit<'a>(
    results: &mut Vec<Result<Found<'a>, VisitError>>,
    nodes: &'a [Node],
    segments: &[(usize, usize)],
    path: &str,
) {
    let root = match nodes.first() {
        Some(node) => node,
        None => return results.push(Err(VisitError::RootNotFound)),
    };

    if let Some(range) = segments.first() {
        results.push(Ok(Found::new(None, None, root.route)));
        visit_node(results, nodes, root, path, segments, range, 1);
    } else {
        results.push(Ok(Found::leaf(None, None, root.route)));
        #[rustfmt::skip]
        results.extend(root.entries().filter_map(|key| match nodes.get(*key) {
            Some(n @ Node { pattern: Pattern::Wildcard(param), .. }) => {
                Some(Ok(Found::leaf(Some(param), None, n.route)))
            }
            Some(_) => None,
            None => Some(Err(VisitError::NodeNotFound)),
        }));
    }
}

/// Recursively search for descendants of the node at `key` that have a
/// pattern that matches the path segment at `index`. If a match is found,
/// we'll add it to `results` and continue our search with the descendants
/// of matched node against the path segment at next index.
fn visit_node<'a>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Result<Found<'a>, VisitError>>,

    // A reference to the route store that contains the route tree.
    nodes: &'a [Node],

    // A reference to the most recently matched node.
    node: &'a Node,

    // The url path that we are attempting to match against the route tree.
    path: &str,

    segments: &[(usize, usize)],

    // The start and end offset of the current path segment.
    at: &(usize, usize),

    // The index of the next path segment in `self.segments`.
    index: usize,
) {
    // Get the value of the path segment at `range`. We'll eagerly borrow
    // and cache this slice from `path` to avoid having to build the ref
    // for each descendant with a static pattern.
    let segment = &path[at.0..at.1];

    let next = segments.get(index);

    // Search for descendant nodes that match `segment`.
    for option in node.entries().map(|key| nodes.get(*key)) {
        #[rustfmt::skip]
        let (mut found, child) = match option {
            Some(n @ Node { pattern: Pattern::Wildcard(param), .. }) => (
                Found::leaf(Some(param), Some((at.0, path.len())), n.route),
                n,
            ),

            Some(n @ Node { pattern: Pattern::Dynamic(param), .. }) => (
                Found::new(Some(param), Some((at.0, at.1)), n.route),
                n,
            ),

            // The node has a static pattern. Attempt to match the pattern
            // value against the current path segment.
            Some(n @ Node { pattern: Pattern::Static(value), .. }) => {
                if value == segment {
                    (Found::new(None, Some((at.0, at.1)), n.route), n)
                } else {
                    continue;
                }
            }

            Some(Node { pattern: Pattern::Root, .. }) => {
                continue;
            }

            None => {
                results.push(Err(VisitError::NodeNotFound));
                continue;
            }
        };

        if let Some(range) = next {
            results.push(Ok(found));
            visit_node(results, nodes, child, path, segments, range, index + 1);
        } else {
            found.is_leaf = true;
            results.push(Ok(found));
            #[rustfmt::skip]
            results.extend(child.entries().filter_map(|key| match nodes.get(*key) {
                Some(n @ Node { pattern: Pattern::Wildcard(param), .. }) => {
                    Some(Ok(Found::leaf(Some(param), None, n.route)))
                }
                Some(_) => None,
                None => Some(Err(VisitError::NodeNotFound)),
            }));
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

impl<'a> Found<'a> {
    #[inline]
    fn new(param: Option<&'a str>, at: Option<(usize, usize)>, route: Option<usize>) -> Self {
        Self {
            is_leaf: false,
            route,
            param,
            at,
        }
    }

    #[inline]
    fn leaf(param: Option<&'a str>, at: Option<(usize, usize)>, route: Option<usize>) -> Self {
        Self {
            is_leaf: true,
            route,
            param,
            at,
        }
    }
}
