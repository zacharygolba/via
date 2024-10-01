use std::vec::IntoIter;

use crate::routes::RouteStore;
use crate::visitor::Visited;

/// An iterator over the routes that match a given path.
///
pub struct Visit<'a, T> {
    store: &'a RouteStore<T>,
    iter: IntoIter<Visited>,
}

impl<'a, T> Visit<'a, T> {
    pub(crate) fn new(store: &'a RouteStore<T>, iter: IntoIter<Visited>) -> Self {
        Self { store, iter }
    }
}

impl<'a, T> Iterator for Visit<'a, T> {
    type Item = (Option<&'a T>, Option<&'static str>, Visited);

    fn next(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next()?;
        let store = self.store;
        let node = store.get(visited.key);

        Some((
            node.route.and_then(|key| store.route(key)),
            node.param(),
            visited,
        ))
    }
}

impl<'a, T> DoubleEndedIterator for Visit<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next_back()?;
        let store = self.store;
        let node = store.get(visited.key);

        Some((
            node.route.and_then(|key| store.route(key)),
            node.param(),
            visited,
        ))
    }
}
