use std::vec::IntoIter;

use crate::routes::RouteStore;
use crate::visitor::Found;

/// An iterator over the nodes that match a uri path.
///
pub struct Visit<'a, T> {
    store: &'a RouteStore<T>,
    iter: IntoIter<(Option<usize>, Found)>,
}

impl<'a, T> Visit<'a, T> {
    pub(crate) fn new(store: &'a RouteStore<T>, iter: IntoIter<(Option<usize>, Found)>) -> Self {
        Self { store, iter }
    }
}

impl<'a, T> Iterator for Visit<'a, T> {
    type Item = (Option<&'a T>, Found);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, found) = self.iter.next()?;
        let route = key.and_then(|k| self.store.route(k));

        Some((route, found))
    }
}

impl<'a, T> DoubleEndedIterator for Visit<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (key, found) = self.iter.next_back()?;
        let route = key.and_then(|k| self.store.route(k));

        Some((route, found))
    }
}
