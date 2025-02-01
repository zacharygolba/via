use crate::path::{Param, Pattern, Split};
use crate::router::Node;

/// A matched node in the route tree.
///
/// Contains a reference to the route associated with the node and additional
/// metadata about the match.
///
#[derive(Debug)]
pub struct Found<'a, T> {
    /// True if there were no more segments to match against the children of
    /// the matched node. Otherwise, false.
    ///
    pub exact: bool,

    /// The name of the dynamic parameter that matched the path segment.
    ///
    pub param: Option<&'a Param>,

    /// The start and end offset of the parameter that matched the path
    /// segment.
    ///
    pub range: Option<[usize; 2]>,

    /// The key of the route associated with the node that matched the path
    /// segment.
    ///
    pub route: Option<&'a T>,
}

#[derive(Clone, Debug)]
pub struct Match {
    value: usize,
    range: Option<[usize; 2]>,
}

impl Match {
    #[inline]
    fn found(exact: bool, key: usize, range: Option<[usize; 2]>) -> Self {
        Self {
            range,
            value: (key << 2) | (1 << 0) | (if exact { 1 } else { 0 } << 1),
        }
    }

    #[inline]
    fn not_found() -> Self {
        Self {
            value: 0,
            range: None,
        }
    }
}

impl Match {
    #[inline]
    pub(crate) fn try_load(self) -> Option<(bool, usize, Option<[usize; 2]>)> {
        let Self { range, value } = self;

        if value & 0b01 != 0 {
            Some(((value & 0b10) != 0, value >> 2, range))
        } else {
            None
        }
    }
}

pub fn search<T>(nodes: &[Node<T>], path: &str) -> Vec<Match> {
    let (key, children) = match nodes.first() {
        Some(node) => (0, &node.children),
        None => return vec![Match::not_found()],
    };

    let mut results = Vec::with_capacity(8);
    let mut segments = Vec::with_capacity(8);

    for range in Split::new(path) {
        segments.push(range);
    }

    results.push(Match::found(segments.is_empty(), key, None));

    if let Some(match_next) = &children {
        rsearch(
            &mut results,
            nodes,
            match_next,
            path,
            &segments,
            key,
            segments.get(key),
        );
    }

    results
}

/// Recursively search for nodes that match the uri path.
fn rsearch<T>(
    // A mutable reference to a vector that contains the matches that we
    // have found so far.
    results: &mut Vec<Match>,

    // A reference to the route store that contains the route tree.
    nodes: &[Node<T>],

    // A slice containing the indices of the nodes to match against the current
    // path segment at `index`.
    match_now: &[usize],

    // A str containing the entire url path.
    path: &str,

    // A reference to the range of each segment separated by / in `path`.
    segments: &[[usize; 2]],

    // The index of the path segment to match against `match_now` in `segments`.
    index: usize,

    range: Option<&[usize; 2]>,
) {
    for key in match_now {
        let (pattern, children) = match nodes.get(*key) {
            Some(node) => (&node.pattern, &node.children),
            None => {
                results.push(Match::not_found());
                continue;
            }
        };

        let (index, range) = match pattern {
            Pattern::Static(name) => match range {
                // The node has a static pattern that matches the path segment.
                Some(at) if name == &path[at[0]..at[1]] => {
                    let next_index = index + 1;
                    let next_range = segments.get(next_index);

                    results.push(Match::found(next_range.is_none(), *key, None));
                    (next_index, next_range)
                }
                Some(_) | None => continue,
            },

            // The node has a dynamic pattern that can match any value.
            Pattern::Dynamic(_) => {
                let next_index = index + 1;
                let next_range = segments.get(next_index);

                results.push(Match::found(next_range.is_none(), *key, range.copied()));
                (next_index, next_range)
            }

            // The node has a wildcard pattern that can match any value
            // and consume the remainder of the uri path.
            Pattern::Wildcard(_) => {
                let range = range.map(|at| [at[0], path.len()]);
                results.push(Match::found(true, *key, range));
                continue;
            }

            // A root node cannot be an edge. If this branch is matched, it is
            // indicative of a bug in this crate.
            Pattern::Root => {
                // Placeholder for tracing...
                continue;
            }
        };

        if let Some(match_next) = children {
            rsearch(results, nodes, match_next, path, segments, index, range);
        }
    }
}
