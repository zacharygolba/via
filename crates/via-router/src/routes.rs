use core::slice::Iter;

use crate::{
    path::{Param, Pattern},
    Router,
};

/// A node in the route tree that represents a single path segment.
pub struct Node {
    /// The pattern used to match the node against a path segment.
    pub pattern: Pattern,

    /// The index of the route in the route store associated with the node.
    pub route: Option<usize>,

    /// The indices of the nodes that are reachable from the current node.
    entries: Vec<usize>,
}

/// A mutable representation of a single node the route store. This type is used
/// to modify the `entries` field field of the node at `key` while keeping the
/// internal state of the route store consistent.
pub struct RouteEntry<'a, T> {
    /// A mutable reference to the route store that contains the node.
    router: &'a mut Router<T>,

    /// The key of the node that we are currently working with.
    key: usize,
}

impl Node {
    pub fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            entries: Vec::new(),
            route: None,
        }
    }

    /// Returns an iterator that yields the indices of the nodes that are
    /// reachable from `self`.
    pub fn entries(&self) -> Iter<usize> {
        self.entries.iter()
    }

    /// Returns an optional reference to the name of the dynamic parameter
    /// associated with the node. The returned value will be `None` if the
    /// node has a `Root` or `Static` pattern.
    pub fn param(&self) -> Option<&Param> {
        match &self.pattern {
            Pattern::Wildcard(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }
}

impl Node {
    /// Pushes a new key into the entries of the node and return it.
    fn push(&mut self, key: usize) -> usize {
        self.entries.push(key);
        key
    }
}

impl<'a, T> RouteEntry<'a, T> {
    pub fn new(router: &'a mut Router<T>, key: usize) -> Self {
        Self { router, key }
    }

    /// Pushes a new node into the store and associates the index of the new
    /// node to the entries of the current node. Returns the index of the
    /// newly inserted node.
    pub fn push(&mut self, node: Node) -> usize {
        // Push the node into the store and get the index of the newly inserted
        // node.
        let next_node_index = self.router.push(node);

        // Associate the index of the newly inserted node with the current node
        // by adding the index to the entries of the current node.
        self.router.node_mut(self.key).push(next_node_index);

        // Return the index of the newly inserted node in the store.
        next_node_index
    }

    /// Inserts a new route into the store and associates the index of the new
    /// route to the current node. Returns the index of the newly inserted route.
    pub fn insert_route(&mut self, route: T) -> usize {
        // Push the route into the store and get the index of the newly
        // inserted route.
        let route_index = self.router.push_route(route);

        // Associate the route index with the current node.
        self.router.node_mut(self.key).route = Some(route_index);

        // Return the index of the newly inserted route in the store.
        route_index
    }

    /// Returns the index of the route at the current node. If the node does not
    /// have a route associated with it, a new route will be inserted by calling
    /// the provided closure `f`.
    pub fn get_or_insert_route_with<F>(&mut self, f: F) -> usize
    where
        F: FnOnce() -> T,
    {
        // Get the index of the route associated with the node if it exists.
        let route = self.router.node(self.key).route;

        // If the node does not have a route associated with it, insert a
        // new route into the store, associate it with the node, and
        // return the index of the route in the store.
        route.unwrap_or_else(|| self.insert_route(f()))
    }
}
