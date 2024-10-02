use std::vec::IntoIter;

use crate::path::Param;
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
pub struct Found<'a, T> {
    /// True if there were no more segments to match against the children of the
    /// matched node. Otherwise, false.
    ///
    pub is_leaf: bool,

    /// A reference to the route referenced by the node that matched the path
    /// segment.
    ///
    pub route: Option<&'a T>,

    /// A reference to the name of the dynamic parameter that matched the path
    /// segment.
    ///
    pub param: Option<Param>,

    /// An array containing the start and end index of the path segment that
    /// matched the node containing `route`.
    ///
    pub at: [usize; 2],
}

impl<'a, T> Visit<'a, T> {
    pub(crate) fn new(store: &'a RouteStore<T>, iter: IntoIter<Visited>) -> Self {
        Self { store, iter }
    }
}

impl<'a, T> Iterator for Visit<'a, T> {
    type Item = Found<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next()?;
        let store = self.store;
        let node = store.get(visited.key);

        Some(Found {
            is_leaf: visited.is_leaf,
            route: node.route.and_then(|key| store.route(key)),
            param: node.param().cloned(),
            at: visited.at,
        })
    }
}

impl<'a, T> DoubleEndedIterator for Visit<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next_back()?;
        let store = self.store;
        let node = store.get(visited.key);

        Some(Found {
            is_leaf: visited.is_leaf,
            route: node.route.and_then(|key| store.route(key)),
            param: node.param().cloned(),
            at: visited.at,
        })
    }
}
