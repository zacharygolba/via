use smallvec::SmallVec;
use std::{iter::Peekable, rc::Rc, str::CharIndices};

use crate::node::{Node, Pattern};
use crate::Store;

/// Represents either a partial or exact match for a given path segment.
#[derive(Clone, Copy)]
pub struct Match<'a, 'b, T> {
    /// Indicates whether or not the match is considered an exact match.
    /// If the match is exact, both the middleware and responders will be
    /// called during a request. Otherwise, only the middleware will be
    /// called.
    pub is_exact_match: bool,

    /// The value of the path segment that matches self.pattern(). If the
    /// matched route has a CatchAll pattern, this will be the remainder
    /// of the url path without the leading `/` character.
    pub path_segment: &'b str,

    /// The node that matches `self.value`.
    node: &'a Node<T>,
}

/// An iterator that yields all possible partial and exact matches for a url path.
pub struct Visit<'a, 'b, T> {
    node: &'a Node<T>,
    store: &'a Store<T>,
    depth: usize,
    index: usize,
    path_value: &'b str,
    path_segments: Rc<SmallVec<[(usize, &'b str); 6]>>,
    visitor_delegate: Option<Box<Self>>,
}

/// An iterator of each path segment in a url path.
#[derive(Debug, Clone)]
pub(crate) struct Segments<'a> {
    chars: Peekable<CharIndices<'a>>,
    value: &'a str,
}

impl<'a, 'b, T> Match<'a, 'b, T> {
    /// Returns a key-value pair where key is the name of the dynamic segment
    /// that was matched against and value is `self.value`. If the matched
    /// route does not have any dynamic segments, `None` will be returned.
    pub fn param(&self) -> Option<(&'static str, &'b str)> {
        if let Pattern::CatchAll(name) | Pattern::Dynamic(name) = self.pattern() {
            Some((name, self.path_segment))
        } else {
            None
        }
    }

    pub fn pattern(&self) -> Pattern {
        self.node.pattern
    }

    /// Returns a reference to the route that matches `self.value`.
    pub fn route(&self) -> Option<&'a T> {
        self.node.route.as_ref()
    }
}

impl<'a, 'b, T> Visit<'a, 'b, T> {
    /// Returns a new visitor to begin our search at the root `node` that match
    /// the provided `path`.
    pub(crate) fn new(store: &'a Store<T>, node: &'a Node<T>, path: &'b str) -> Self {
        Visit {
            node,
            store,
            depth: 0,
            index: 0,
            path_value: path,
            path_segments: Rc::new(Segments::new(path).collect()),
            visitor_delegate: None,
        }
    }

    /// Returns a new visitor to search for descendants of `node` that match
    /// the next path segment in `self.path_segments`.
    fn fork(&self, node: &'a Node<T>) -> Box<Self> {
        Box::new(Visit {
            node,
            store: self.store,
            index: 0,
            depth: self.depth + 1,
            path_value: self.path_value,
            path_segments: Rc::clone(&self.path_segments),
            visitor_delegate: None,
        })
    }

    /// Calls next on the visitor delegate and returns the next match if one
    /// exists. If the visitor delegate is exhausted, it will be set to None
    /// to prevent us from attempting to delegate to it again.
    fn delegate_next_match(&mut self) -> Option<Match<'a, 'b, T>> {
        self.visitor_delegate
            .as_mut()
            .and_then(|delegate| delegate.next())
            .or_else(|| {
                self.visitor_delegate = None;
                None
            })
    }

    /// Attempts to find the next immediate decedent that matches `predicate`
    /// starting from the current index and then sets `self.index` to the
    /// index of the next match in `self.node.entries`.
    fn find_next_match<F>(&mut self, predicate: F) -> Option<&'a Node<T>>
    where
        F: FnMut(&'a Node<T>) -> bool,
    {
        match self.node.find(self.store, self.index, predicate) {
            Some((index, next)) => {
                self.index = index + 1;
                Some(next)
            }
            None => {
                self.index = self
                    .node
                    .entries
                    .as_ref()
                    .map_or(0, |entries| entries.len());
                None
            }
        }
    }

    /// Returns the value of the current path segment that we are attempting to
    /// match if it exists. The returned value should only be `None` if we are
    /// attempting to match a root url path (i.e `"/"`).
    fn get_path_segment_value(&self) -> Option<&'b str> {
        self.path_segments.get(self.depth).map(|(_, value)| *value)
    }

    // Returns a reference to remaining path starting from the current path
    // segment without the leading `/` character.
    fn get_remaining_path(&self) -> &'b str {
        self.path_segments
            .get(self.depth)
            .map(|(start, _)| self.path_value[*start..].trim_start_matches('/'))
            .unwrap_or("")
    }
}

impl<'a, 'b, T> Iterator for Visit<'a, 'b, T> {
    type Item = Match<'a, 'b, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // First, we attempt to delegate to the next visitor to see if there
        // are any matches from descendant nodes.
        if let Some(component) = self.delegate_next_match() {
            return Some(component);
        }

        // If we are unable to delegate to the next visitor, we attempt to find
        // a node that matches the current path segment. We'll continue to match
        // against the current path segment until all possible matches at the
        // current depth are exhausted.
        if let Some(path_segment) = self.get_path_segment_value() {
            let mut is_exact_match = self.depth == self.path_segments.len() - 1;
            let mut path_segment = path_segment;
            let node = self.find_next_match(|entry| path_segment == entry.pattern)?;

            if matches!(node.pattern, Pattern::CatchAll(_)) {
                // The next node has a `CatchAll` pattern and will be considered an exact
                // match. This means that both the middleware and the responders will be
                // called for `next` and we will attempt to match the next path segment
                // with descendant nodes.
                is_exact_match = true;
                path_segment = self.get_remaining_path();
            } else {
                // The next node may have descendant that the next path segment. Therefore,
                // we'll fork the current visitor and attempt to delegate our search to
                // the matching node in the next iteration.
                //
                // While it is tempting to change the else condition to `else if !is_exact`,
                // we must consider the case where the next descendant has a `CatchAll`
                // pattern.
                self.visitor_delegate = Some(self.fork(node));
            }

            return Some(Match {
                is_exact_match,
                path_segment,
                node,
            });
        }

        // If there is no path segment to match against, we attempt to find an
        // immediate descendant node with a CatchAll pattern. This is required
        // to support matching the "index" path of a descendant node with a
        // CatchAll pattern.
        let node = self.find_next_match(|entry| matches!(entry.pattern, Pattern::CatchAll(_)))?;

        Some(Match {
            is_exact_match: true,
            path_segment: self.get_remaining_path(),
            node,
        })
    }
}

impl<'a> Segments<'a> {
    pub(crate) fn new(value: &'a str) -> Self {
        Segments {
            chars: value.char_indices().peekable(),
            value,
        }
    }
}

impl Segments<'static> {
    pub(crate) fn patterns(self) -> impl Iterator<Item = Pattern> {
        self.map(|(_, value)| Pattern::from(value))
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = None;
        let mut end = self.value.len();

        while let (index, '/') = *self.chars.peek()? {
            start = Some(index);
            self.chars.next();
        }

        while let Some((index, value)) = self.chars.peek() {
            if *value == '/' {
                end = *index;
                break;
            }

            self.chars.next();
        }

        Some((start?, &self.value[(start? + 1)..end]))
    }
}
