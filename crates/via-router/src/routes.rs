use core::slice::Iter;

use crate::path::{ParamName, Pattern};

/// A node in the route tree that represents a single path segment.
pub struct Node {
    /// The indices of the nodes that are reachable from the current node.
    pub entries: Option<Vec<usize>>,

    /// The pattern used to match the node against a path segment.
    pub pattern: Pattern,

    /// The index of the route in the route store associated with the node.
    pub route: Option<usize>,
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
    nodes: Vec<Node>,

    /// A vector of routes associated with the nodes in the route tree.
    routes: Vec<T>,
}

impl Node {
    pub fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            entries: None,
            route: None,
        }
    }

    /// Returns an iterator that yields the indices of the nodes that are
    /// reachable from `self`.
    pub fn entries(&self) -> Iter<usize> {
        match &self.entries {
            Some(entries) => entries.iter(),
            None => [].iter(),
        }
    }

    /// Returns an optional reference to the name of the dynamic parameter
    /// associated with the node. The returned value will be `None` if the
    /// node has a `Root` or `Static` pattern.
    pub fn param(&self) -> Option<&ParamName> {
        match &self.pattern {
            Pattern::CatchAll(param) | Pattern::Dynamic(param) => Some(param),
            _ => None,
        }
    }
}

impl Node {
    /// Pushes a new key into the entries of the node and return it.
    fn push(&mut self, key: usize) -> usize {
        self.entries.get_or_insert_with(Vec::new).push(key);
        key
    }
}

impl<'a, T> RouteEntry<'a, T> {
    /// Pushes a new node into the store and associates the index of the new
    /// node to the entries of the current node. Returns the index of the
    /// newly inserted node.
    pub fn push(&mut self, node: Node) -> usize {
        // Push the node into the store and get the index of the newly inserted
        // node.
        let next_node_index = self.store.push(node);

        // Associate the index of the newly inserted node with the current node
        // by adding the index to the entries of the current node.
        self.store.get_mut(self.key).push(next_node_index);

        // Return the index of the newly inserted node in the store.
        next_node_index
    }

    /// Inserts a new route into the store and associates the index of the new
    /// route to the current node. Returns the index of the newly inserted route.
    pub fn insert_route(&mut self, route: T) -> usize {
        // Push the route into the store and get the index of the newly
        // inserted route.
        let route_index = self.store.push_route(route);

        // Associate the route index with the current node.
        self.store.get_mut(self.key).route = Some(route_index);

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
        let route = self.store.get(self.key).route;

        // If the node does not have a route associated with it, insert a
        // new route into the store, associate it with the node, and
        // return the index of the route in the store.
        route.unwrap_or_else(|| self.insert_route(f()))
    }
}

impl<T> RouteStore<T> {
    /// Constructs a new, empty `RouteStore`.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            routes: Vec::new(),
        }
    }

    /// Returns a mutable representation of a single node in the route store.
    pub fn entry(&mut self, key: usize) -> RouteEntry<T> {
        RouteEntry { key, store: self }
    }

    /// Pushes a new node into the store and returns the key of the newly
    /// inserted node.
    pub fn push(&mut self, node: Node) -> usize {
        let key = self.nodes.len();

        self.nodes.push(node);
        key
    }

    /// Shrinks the capacity of the route store as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.nodes.shrink_to_fit();
    }

    /// Returns a shared reference to the node at the given `key`.
    pub fn get(&self, key: usize) -> &Node {
        &self.nodes[key]
    }

    /// Returns a mutable reference to the node at the given `key`.
    pub fn get_mut(&mut self, key: usize) -> &mut Node {
        &mut self.nodes[key]
    }

    /// Returns a shared reference to the route at the given `key`.
    ///
    pub fn route(&self, key: usize) -> Option<&T> {
        self.routes.get(key)
    }

    /// Returns a mutable reference to the route at the given `key`.
    ///
    pub fn route_mut(&mut self, key: usize) -> &mut T {
        &mut self.routes[key]
    }
}

impl<T> RouteStore<T> {
    /// Pushes a new route into the store and returns the index of the newly
    /// inserted route.
    fn push_route(&mut self, route: T) -> usize {
        let index = self.routes.len();
        self.routes.push(route);
        index
    }
}
