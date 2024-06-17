use std::{
    fmt::{self, Debug},
    ops::{Index, IndexMut},
};

use crate::node::Node;

#[derive(Clone)]
pub(crate) struct RouteStore<T> {
    entries: Vec<Node<T>>,
}

#[derive(Debug)]
pub(crate) struct RouteEntry<'a, T> {
    pub(crate) key: usize,
    routes: &'a mut RouteStore<T>,
}

impl<T> RouteStore<T> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::with_capacity(256),
        }
    }

    pub(crate) fn entry(&mut self, key: usize) -> RouteEntry<T> {
        RouteEntry { key, routes: self }
    }

    pub(crate) fn insert(&mut self, node: Node<T>) -> usize {
        let next_key = self.entries.len();

        self.entries.push(node);
        next_key
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
        &mut self.routes[self.key]
    }
}
