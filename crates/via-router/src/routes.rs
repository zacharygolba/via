use slab::Slab;
use std::slice;

use crate::path::Pattern;

#[derive(Clone, Debug)]
pub(crate) struct Node<T> {
    pub(crate) entries: Option<Vec<usize>>,
    pub(crate) pattern: Pattern,
    pub(crate) route: Option<Box<T>>,
}

pub(crate) struct RouteStore<T> {
    entries: Slab<Box<Node<T>>>,
}

pub(crate) struct RouteEntry<'a, T> {
    pub(crate) index: usize,
    routes: &'a mut RouteStore<T>,
}

impl<T> Node<T> {
    pub(crate) fn new(pattern: Pattern) -> Self {
        Self {
            entries: None,
            route: None,
            pattern,
        }
    }

    pub(crate) fn entries(&self) -> slice::Iter<usize> {
        if let Some(entries) = self.entries.as_ref() {
            entries.iter()
        } else {
            [].iter()
        }
    }

    pub(crate) fn push(&mut self, index: usize) -> usize {
        self.entries.get_or_insert_with(Vec::new).push(index);
        index
    }
}

impl<T> RouteStore<T> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Slab::with_capacity(256),
        }
    }

    pub(crate) fn entry(&mut self, index: usize) -> RouteEntry<T> {
        RouteEntry {
            index,
            routes: self,
        }
    }

    pub(crate) fn get(&self, index: usize) -> &Node<T> {
        self.entries.get(index).unwrap()
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> &mut Node<T> {
        self.entries.get_mut(index).unwrap()
    }

    pub(crate) fn insert(&mut self, node: Box<Node<T>>) -> usize {
        self.entries.insert(node)
    }
}

impl<'a, T> RouteEntry<'a, T> {
    pub(crate) fn insert(&mut self, node: Box<Node<T>>) -> usize {
        let index = self.routes.insert(node);
        self.routes.get_mut(self.index).push(index)
    }
}
