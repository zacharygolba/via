use std::{iter::IntoIterator, slice};

use crate::routes::RouteStore;

#[derive(Clone, Debug)]
pub struct Node<T> {
    pub(crate) entries: Option<Vec<usize>>,
    pub(crate) pattern: Pattern,
    pub(crate) route: Option<T>,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pattern {
    CatchAll(&'static str),
    Dynamic(&'static str),
    Static(&'static str),
    Root,
}

impl<T> Node<T> {
    pub fn route(&self) -> Option<&T> {
        self.route.as_ref()
    }

    pub fn route_mut(&mut self) -> &mut Option<T> {
        &mut self.route
    }

    pub(crate) fn new(pattern: Pattern) -> Self {
        Node {
            pattern,
            entries: None,
            route: None,
        }
    }

    pub(crate) fn find<'a, F>(
        &'a self,
        store: &'a RouteStore<T>,
        from_index: usize,
        mut predicate: F,
    ) -> Option<(usize, &'a Node<T>)>
    where
        F: FnMut(&'a Node<T>) -> bool,
    {
        self.iter()
            .skip(from_index)
            .enumerate()
            .find_map(|(index, key)| {
                let node = &store[*key];
                if predicate(node) {
                    Some((from_index + index, node))
                } else {
                    None
                }
            })
    }

    pub(crate) fn iter(&self) -> slice::Iter<usize> {
        if let Some(entries) = self.entries.as_ref() {
            entries.iter()
        } else {
            [].iter()
        }
    }

    pub(crate) fn push(&mut self, key: usize) {
        self.entries
            .get_or_insert_with(|| Vec::with_capacity(4))
            .push(key);
    }
}

impl From<&'static str> for Pattern {
    fn from(value: &'static str) -> Pattern {
        match value.chars().next() {
            Some('*') => Pattern::CatchAll(&value[1..]),
            Some(':') => Pattern::Dynamic(&value[1..]),
            _ => Pattern::Static(value),
        }
    }
}

impl PartialEq<&str> for Pattern {
    fn eq(&self, other: &&str) -> bool {
        if let Pattern::Static(value) = *self {
            value == *other
        } else {
            true
        }
    }
}

impl PartialEq<Pattern> for &str {
    fn eq(&self, other: &Pattern) -> bool {
        other == self
    }
}
