use slab::Slab;
use std::slice;

use crate::path::Pattern;

pub struct Node<T> {
    pub entries: Option<Vec<usize>>,
    pub pattern: Pattern,
    pub route: Option<Box<T>>,
}

pub struct RouteStore<T> {
    entries: Slab<Box<Node<T>>>,
}

pub struct RouteEntry<'a, T> {
    pub index: usize,
    routes: &'a mut RouteStore<T>,
}

impl<T> Node<T> {
    pub fn new(pattern: Pattern) -> Self {
        Self {
            entries: None,
            route: None,
            pattern,
        }
    }

    pub fn entries(&self) -> slice::Iter<usize> {
        if let Some(entries) = self.entries.as_ref() {
            entries.iter()
        } else {
            [].iter()
        }
    }

    pub fn push(&mut self, index: usize) -> usize {
        self.entries.get_or_insert_with(Vec::new).push(index);
        index
    }
}

impl<T> RouteStore<T> {
    pub fn new() -> Self {
        Self {
            entries: Slab::with_capacity(256),
        }
    }

    pub fn entry(&mut self, index: usize) -> RouteEntry<T> {
        RouteEntry {
            index,
            routes: self,
        }
    }

    pub fn get(&self, index: usize) -> &Node<T> {
        self.entries.get(index).unwrap()
    }

    pub fn get_mut(&mut self, index: usize) -> &mut Node<T> {
        self.entries.get_mut(index).unwrap()
    }

    pub fn insert(&mut self, node: Box<Node<T>>) -> usize {
        self.entries.insert(node)
    }
}

impl<'a, T> RouteEntry<'a, T> {
    pub fn insert(&mut self, node: Box<Node<T>>) -> usize {
        let index = self.routes.insert(node);
        self.routes.get_mut(self.index).push(index)
    }
}
