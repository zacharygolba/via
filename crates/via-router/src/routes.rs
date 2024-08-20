use core::slice;

use crate::path::Pattern;

/// A node in the route tree that represents a single path segment.
pub struct Node<T> {
    /// The indices of the nodes that are reachable from the current node.
    pub entries: Option<Vec<usize>>,

    /// The pattern used to match the node against a path segment.
    pub pattern: Pattern,

    /// The index of the route in the route store associated with the node.
    route: Option<Box<T>>,
}

/// A mutable representation of a single node the route store. This type is used
/// to modify the `entries` field field of the node at `key` while keeping the
/// internal state of the route store consistent.
pub struct RouteEntry<'a, T> {
    /// The key of the node that we are currently working with.
    key: usize,

    /// A mutable reference to the route store that contains the node.
    store: &'a mut RouteStore<T>,
}

/// A container type used to improve the cache locality of nodes and routes in
/// the route tree.
pub struct RouteStore<T> {
    /// A collection of nodes that represent the path segments of a route.
    nodes: Vec<Node<T>>,
}

impl<T> Node<T> {
    pub fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            entries: None,
            route: None,
        }
    }

    /// Returns an iterator that yields the indices of the nodes that are
    /// reachable from `self`.
    pub fn entries(&self) -> slice::Iter<usize> {
        match &self.entries {
            Some(entries) => entries.iter(),
            None => [].iter(),
        }
    }

    /// Returns an optional reference to the name of the dynamic parameter
    /// associated with the node. The returned value will be `None` if the
    /// node has a `Root` or `Static` pattern.
    pub fn param(&self) -> Option<&'static str> {
        match &self.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }

    /// Returns an optional reference to the route associated with the node.
    pub fn route(&self) -> Option<&T> {
        match &self.route {
            Some(route) => Some(route),
            None => None,
        }
    }

    /// Returns a mutable reference to the `route` field of the node.
    pub fn route_mut(&mut self) -> &mut Option<Box<T>> {
        &mut self.route
    }
}

impl<T> Node<T> {
    /// Pushes a new index into the entries of the node and returns the index.
    fn push(&mut self, index: usize) -> usize {
        self.entries.get_or_insert_with(Vec::new).push(index);
        index
    }
}

impl<'a, T> RouteEntry<'a, T> {
    /// Pushes a new node into the store and associates the index of the new
    /// node to the entries of the current node. Returns the index of the
    /// newly inserted node.
    pub fn push(&mut self, node: Node<T>) -> usize {
        // Push the node into the store and get the index of the newly inserted
        // node.
        let next_node_index = self.store.push(node);

        // Associate the index of the newly inserted node with the current node
        // by adding the index to the entries of the current node.
        self.store.get_mut(self.key).push(next_node_index);

        // Return the index of the newly inserted node in the store.
        next_node_index
    }
}

impl<T> RouteStore<T> {
    /// Constructs a new, empty `RouteStore`.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Returns a mutable representation of a single node in the route store.
    pub fn entry(&mut self, index: usize) -> RouteEntry<T> {
        RouteEntry {
            key: index,
            store: self,
        }
    }

    /// Pushes a new node into the store and returns the index of the newly
    /// inserted node.
    pub fn push(&mut self, node: Node<T>) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    /// Returns a shared reference to the node at the given index.
    pub fn get(&self, index: usize) -> &Node<T> {
        self.nodes.get(index).unwrap()
    }

    /// Returns a mutable reference to the node at the given index.
    pub fn get_mut(&mut self, index: usize) -> &mut Node<T> {
        self.nodes.get_mut(index).unwrap()
    }
}
