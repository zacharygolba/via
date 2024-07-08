use std::{
    fmt::{self, Debug},
    ops::{Index, IndexMut},
    slice,
};

use crate::path::Pattern;

#[derive(Clone, Debug)]
pub(crate) struct Node<T> {
    pub(crate) entries: Option<Vec<usize>>,
    pub(crate) pattern: Pattern,
    pub(crate) route: Option<T>,
}

#[derive(Clone)]
pub(crate) struct RouteStore<T> {
    entries: Vec<Node<T>>,
}

#[derive(Debug)]
pub(crate) struct RouteEntry<'a, T> {
    pub(crate) index: usize,
    routes: &'a mut RouteStore<T>,
}

impl<T> Node<T> {
    pub(crate) fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            entries: None,
            route: None,
        }
    }

    pub(crate) fn entries(&self) -> slice::Iter<usize> {
        if let Some(entries) = self.entries.as_ref() {
            entries.iter()
        } else {
            [].iter()
        }
    }

    pub(crate) fn push(&mut self, index: usize) {
        self.entries
            .get_or_insert_with(|| Vec::with_capacity(4))
            .push(index);
    }
}

impl<T> RouteStore<T> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::with_capacity(256),
        }
    }

    pub(crate) fn entry(&mut self, index: usize) -> RouteEntry<T> {
        RouteEntry {
            index,
            routes: self,
        }
    }

    pub(crate) fn insert(&mut self, node: Node<T>) -> usize {
        let next_index = self.entries.len();

        self.entries.push(node);
        next_index
    }
}

impl<T: Debug> Debug for RouteStore<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.entries, f)
    }
}

impl<T> Index<usize> for RouteStore<T> {
    type Output = Node<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<T> IndexMut<usize> for RouteStore<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl<'a, T> RouteEntry<'a, T> {
    pub(crate) fn insert(&mut self, node: Node<T>) -> usize {
        let key = self.routes.insert(node);

        self.node_mut().push(key);
        key
    }

    pub(crate) fn node_mut(&mut self) -> &mut Node<T> {
        &mut self.routes[self.index]
    }
}
