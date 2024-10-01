use std::vec::IntoIter;

use crate::routes::RouteStore;
use crate::visitor::Visit;

/// An iterator over the routes that match a given path.
///
pub struct Matches<'a, T> {
    store: &'a RouteStore<T>,
    iter: IntoIter<Visit>,
}

impl<'a, T> Matches<'a, T> {
    pub(crate) fn new(store: &'a RouteStore<T>, iter: IntoIter<Visit>) -> Self {
        Self { store, iter }
    }
}

impl<'a, T> Iterator for Matches<'a, T> {
    type Item = (Option<&'a T>, Option<&'static str>, Visit);

    fn next(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next()?;
        let store = self.store;
        let node = store.get(visited.key);

        Some((
            node.route.map(|key| store.route(key)),
            node.param(),
            visited,
        ))
    }
}

impl<'a, T> DoubleEndedIterator for Matches<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let visited = self.iter.next_back()?;
        let store = self.store;
        let node = store.get(visited.key);

        Some((
            node.route.map(|key| store.route(key)),
            node.param(),
            visited,
        ))
    }
}
