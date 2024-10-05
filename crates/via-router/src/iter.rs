use std::vec::IntoIter;

use crate::path::{Param, Span};
use crate::routes::RouteStore;
use crate::visitor::Visited;

/// An iterator over the nodes that match a uri path.
///
pub struct Visit<'a, T> {
    store: &'a RouteStore<T>,
    iter: IntoIter<Visited>,
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

    /// A reference to the name of the dynamic parameter that matched the path
    /// segment.
    ///
    pub param: Option<Param>,

    /// An array containing the start and end index of the path segment that
    /// matched the node containing `route`.
    ///
    pub at: Span,
}

impl<'a, T> Visit<'a, T> {
    pub(crate) fn new(store: &'a RouteStore<T>, iter: IntoIter<Visited>) -> Self {
        Self { store, iter }
    }
}

impl<'a, T> Iterator for Visit<'a, T> {
    type Item = (Option<&'a T>, Found);

    fn next(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next()?;
        let store = self.store;
        let node = store.get(visited.key);

        let route = node.route.and_then(|key| store.route(key));
        let found = Found {
            is_leaf: visited.is_leaf,
            param: node.param().cloned(),
            at: visited.at,
        };

        Some((route, found))
    }
}

impl<'a, T> DoubleEndedIterator for Visit<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next_back()?;
        let store = self.store;
        let node = store.get(visited.key);

        let route = node.route.and_then(|key| store.route(key));
        let found = Found {
            is_leaf: visited.is_leaf,
            param: node.param().cloned(),
            at: visited.at,
        };

        Some((route, found))
    }
}
