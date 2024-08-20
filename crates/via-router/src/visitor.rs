use crate::path::Pattern;
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

pub struct Visitor<'a, 'b, T> {
    /// The url path that we are attempting to match against the route tree.
    path_value: &'b str,

    /// A slice of tuples that contain the start and end offset of each path
    /// segment in `self.path_value`.
    segments: &'b [(usize, usize)],

    /// A reference to the route store that contains the route tree.
    store: &'a RouteStore<T>,
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

impl<'a, 'b, T> Visitor<'a, 'b, T> {
    pub fn new(
        path_value: &'b str,
        segments: &'b [(usize, usize)],
        store: &'a RouteStore<T>,
    ) -> Self {
        Self {
            path_value,
            segments,
            store,
        }
    }

    pub fn visit(&self, results: &mut Vec<Match<'a, T>>) {
        // The root node is a special case that we always consider a match.
        results.push(Match {
            // If there are no path segments to match against, we consider the root
            // node to be an exact match.
            exact: self.segments.is_empty(),
            // The root node cannot have parameters.
            param: None,
            // The root node's path segment range should produce to an empty str.
            range: (0, 0),
            route: self.store.get(0).route(),
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
        results: &mut Vec<Match<'a, T>>,
        // The start and end offset of the path segment at `index` in
        // `self.path_value`.
        range: (usize, usize),
        // The index of the path segment in `self.segments` that we are matching
        // against the node at `key`.
        index: usize,
        // The key of the parent node that contains the descendants that we are
        // attempting to match against the path segment at `index`.
        key: usize,
    ) {
        // Get the value of the path segment at `index`. We'll eagerly borrow
        // and cache this slice from `self.path_value` to avoid having to build
        // the reference for each descendant with a `Static` pattern.
        let path_segment = &self.path_value[range.0..range.1];
        // Eagerly calculate and store the next index to avoid having to do so
        // for each descendant with a `Dynamic` or `Static` pattern.
        let next_index = index + 1;
        // Use the value of `next_index` to determine if we are working with the
        // last path segment in `self.segments`. If so, we'll consider any
        // matching descendant to be an exact match. We perform this check
        // eagerly to avoid having to do so for each descendant with a
        // `Dynamic` or `Static` pattern.
        let exact = next_index == self.segments.len();

        // Iterate over the keys of the descendants of the node at `key`.
        for next_key in self.store.get(key).entries().copied() {
            // Get the node at `next_key` from the route store.
            let descendant = self.store.get(next_key);

            // Check if `descendant` has a pattern that matches `path_segment`.
            match descendant.pattern {
                Pattern::Static(value) if value == path_segment => {
                    // The next node has a `Static` pattern that matches the value
                    // of the path segment.
                    results.push(Match {
                        exact,
                        range,
                        param: None,
                        route: descendant.route(),
                    });

                    self.visit_node(results, next_index, next_key);
                }
                Pattern::Dynamic(param) => {
                    // The next node has a `Dynamic` pattern. Therefore, we consider
                    // it a match regardless of the value of the path segment.
                    results.push(Match {
                        exact,
                        range,
                        param: Some(param),
                        route: descendant.route(),
                    });

                    self.visit_node(results, next_index, next_key);
                }
                Pattern::CatchAll(param) => {
                    // The next node has a `CatchAll` pattern and will be considered
                    // an exact match. Due to the nature of `CatchAll` patterns, we
                    // do not have to continue searching for descendants of this
                    // node that match the remaining path segments.
                    results.push(Match {
                        // `CatchAll` patterns are always considered an exact match.
                        exact: true,
                        // The end offset of `path_segment_range` should be the end
                        // offset of the last path segment in the url path since
                        // `CatchAll` patterns match the entire remainder of the
                        // url path from which they are matched.
                        range: (range.0, self.path_value.len()),
                        param: Some(param),
                        route: descendant.route(),
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
    fn visit_node(&self, results: &mut Vec<Match<'a, T>>, index: usize, key: usize) {
        // Check if there is a path segment at `index` to match against
        if let Some(range) = self.segments.get(index).copied() {
            return self.visit_descendants(results, range, index, key);
        }

        // Perform a shallow search for descendants of the current node that
        // have a `CatchAll` pattern. This is required to support matching the
        // "index" path of a descendant node with a `CatchAll` pattern.
        for next_key in self.store.get(key).entries().copied() {
            // Get the node at `next_key` from the route store.
            let descendant = self.store.get(next_key);

            // Check if `descendant` has a `CatchAll` pattern.
            if let Pattern::CatchAll(param) = descendant.pattern {
                // Add the matching node to the vector of matches and continue to
                // search for adjacent nodes with a `CatchAll` pattern.
                results.push(Match {
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
                    route: descendant.route(),
                });
            }
        }
    }
}
