use crate::path::Pattern;
use crate::routes::RouteStore;
use crate::stack_vec::StackVec;

#[derive(Debug)]
pub struct Visited {
    /// An array containing the start and end index of the path segment that
    /// matches self.
    ///
    pub range: [usize; 2],

    /// A boolean that indicates if there could be descendants that also match
    /// the uri path.
    ///
    pub was_leaf: bool,

    /// The route store key of the node that matches the path segement at
    /// the match range.
    ///
    pub(crate) key: usize,
}

pub struct Visitor<'a, 'b, T> {
    /// The url path that we are attempting to match against the route tree.
    path_value: &'b str,

    /// A slice of tuples that contain the start and end offset of each path
    /// segment in `self.path_value`.
    segments: &'b StackVec<[usize; 2], 5>,

    /// A reference to the route store that contains the route tree.
    store: &'a RouteStore<T>,
}

impl<'a, 'b, T> Visitor<'a, 'b, T> {
    pub fn new(
        path: &'b str,
        store: &'a RouteStore<T>,
        segments: &'b StackVec<[usize; 2], 5>,
    ) -> Self {
        Self {
            path_value: path,
            segments,
            store,
        }
    }

    pub fn visit(self, results: &mut Vec<Visited>) {
        // The root node is a special case that we always consider a match.
        results.push(Visited {
            // The root node's key is always `0`.
            key: 0,
            // The root node's path segment range should produce to an empty str.
            range: [0, 0],
            // If there are no path segments to match against, we consider the root
            // node to be an exact match.
            was_leaf: self.segments.is_empty(),
        });

        // Begin the search for matches recursively starting with descendants of
        // the root node.
        self.visit_node(results, 0, 0)
    }
}

impl<'a, 'b, T> Visitor<'a, 'b, T> {
    /// Recursively search for descendants of the node at `key` that have a
    /// pattern that matches the path segment at `index`. If a match is found,
    /// we'll add it to `results` and continue our search with the descendants
    /// of matched node against the path segment at next index.
    fn visit_descendants(
        &self,
        // A mutable reference to a vector that contains the matches that we
        // have found so far.
        results: &mut Vec<Visited>,
        // The start and end offset of the path segment at `index` in
        // `self.path_value`.
        range: [usize; 2],
        // The index of the path segment in `self.segments` that we are matching
        // against the node at `key`.
        index: usize,
        // The key of the parent node that contains the descendants that we are
        // attempting to match against the path segment at `index`.
        key: usize,
    ) {
        let path_value = self.path_value;

        // Get the value of the path segment at `index`. We'll eagerly borrow
        // and cache this slice from `self.path_value` to avoid having to build
        // the reference for each descendant with a `Static` pattern.
        let path_segment = {
            let [start, end] = range;
            path_value.get(start..end).unwrap_or("")
        };

        // Eagerly calculate and store the next index to avoid having to do so
        // for each descendant with a `Dynamic` or `Static` pattern.
        let next_index = index + 1;

        // Use the value of `next_index` to determine if we are working with the
        // last path segment in `self.segments`. If so, we'll consider any
        // matching descendant to be a leaf node. We perform this check eagerly
        // to avoid having to do so for each descendant with a `Dynamic` or
        // `Static` pattern.
        let was_leaf = next_index == self.segments.len();

        let store = self.store;

        // Iterate over the keys of the descendants of the node at `key`.
        for next_key in store.get(key).entries().copied() {
            // Get the node at `next_key` from the route store.
            let descendant = store.get(next_key);

            // Check if `descendant` has a pattern that matches `path_segment`.
            match &descendant.pattern {
                Pattern::Static(value) if path_segment == value.as_str() => {
                    // The next node has a `Static` pattern that matches the value
                    // of the path segment.
                    results.push(Visited {
                        key: next_key,
                        range,
                        was_leaf,
                    });

                    self.visit_node(results, next_index, next_key);
                }
                Pattern::Dynamic(_) => {
                    // The next node has a `Dynamic` pattern. Therefore, we consider
                    // it a match regardless of the value of the path segment.
                    results.push(Visited {
                        key: next_key,
                        range,
                        was_leaf,
                    });

                    self.visit_node(results, next_index, next_key);
                }
                Pattern::CatchAll(_) => {
                    // The next node has a `CatchAll` pattern and will be considered
                    // an exact match. Due to the nature of `CatchAll` patterns, we
                    // do not have to continue searching for descendants of this
                    // node that match the remaining path segments.
                    results.push(Visited {
                        key: next_key,
                        // The end offset of `path_segment_range` should be the end
                        // offset of the last path segment in the url path since
                        // `CatchAll` patterns match the entire remainder of the
                        // url path from which they are matched.
                        range: [range[0], path_value.len()],
                        // `CatchAll` patterns are always considered a leaf node.
                        was_leaf: true,
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
    fn visit_node(&self, results: &mut Vec<Visited>, index: usize, key: usize) {
        // Check if there is a path segment at `index` to match against
        if let Some(range) = self.segments.get(index) {
            return self.visit_descendants(results, range, index, key);
        }

        let store = self.store;

        // Perform a shallow search for descendants of the current node that
        // have a `CatchAll` pattern. This is required to support matching the
        // "index" path of a descendant node with a `CatchAll` pattern.
        for next_key in store.get(key).entries().copied() {
            // Get the node at `next_key` from the route store.
            let descendant = store.get(next_key);

            // Check if `descendant` has a `CatchAll` pattern.
            if let Pattern::CatchAll(_) = descendant.pattern {
                // Add the matching node to the vector of matches and continue to
                // search for adjacent nodes with a `CatchAll` pattern.
                results.push(Visited {
                    key: next_key,
                    // Due to the fact we are looking for `CatchAll` patterns as
                    // an immediate descendant of a node that we consider a match,
                    // we can safely assume that the path segment range should
                    // always produce an empty str.
                    range: [0, 0],
                    // `CatchAll` patterns are always considered an exact match.
                    was_leaf: true,
                });
            }
        }
    }
}
