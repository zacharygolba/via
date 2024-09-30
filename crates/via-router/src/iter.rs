use core::slice::Iter;
use std::vec::IntoIter;

use crate::routes::RouteStore;
use crate::visitor::Visit;

/// Represents either a partial or exact match for a given path segment.
///
pub struct Match<'a, T> {
    /// Indicates whether or not the match is considered an exact match.
    /// If the match is exact, both the middleware and responders will be
    /// called during a request. Otherwise, only the middleware will be
    /// called.
    pub exact: bool,

    /// An optional tuple containing the name of the dynamic segment that
    /// matched the path segment as well as the start and end offset of the
    /// path segment value.
    ///
    pub param: Option<(&'static str, [usize; 2])>,

    /// The route that matches the path segement at `self.range`.
    ///
    pub route: Option<&'a T>,
}

/// An iterator over the routes that match a given path.
///
pub struct Matches<'a, T> {
    store: &'a RouteStore<T>,
    iter: IntoIter<Visit>,
}

impl<'a, T> Match<'a, Vec<T>> {
    /// Returns an iterator that yields a reference to each item in the matched
    /// route.
    pub fn iter(&self) -> Iter<'a, T> {
        match self.route {
            Some(route) => route.iter(),
            None => [].iter(),
        }
    }
}

impl<'a, T> Matches<'a, T> {
    pub(crate) fn new(store: &'a RouteStore<T>, iter: IntoIter<Visit>) -> Self {
        Self { store, iter }
    }
}

impl<'a, T> Iterator for Matches<'a, T> {
    type Item = Match<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next()?;
        let node = self.store.get(next.key);

        Some(Match {
            exact: next.exact,
            param: node.param().zip(Some(next.range)),
            route: node.route.map(|key| self.store.route(key)),
        })
    }
}
