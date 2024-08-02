use std::slice;

use crate::path::Pattern;

/// A node in the route tree that represents a single path segment.
pub struct Node {
    /// The indices of the nodes that are reachable from the current node.
    pub entries: Option<Vec<usize>>,

    /// The pattern used to match the node against a path segment.
    pub pattern: Pattern,

    /// The index of the route in the route store associated with the node.
    pub route: Option<usize>,
}

/// A container type used to improve the cache locality of nodes and routes in
/// the route tree.
pub struct RouteStore<T> {
    /// A collection of nodes that represent the path segments of a route.
    nodes: Vec<Node>,

    /// A collection of routes that are associated with the nodes stored in
    /// `self.nodes`.
    routes: Vec<Box<T>>,
}

/// A mutable representation of a single node the route store. This type is used
/// to modify the `entries` field or `route` field of the node at `node_index`
/// while keeping the internal state of the route store consistent.
pub struct RouteEntry<'a, T> {
    /// The index of the node that we are currently working with.
    node_index: usize,

    /// A mutable reference to the route store that contains the node.
    route_store: &'a mut RouteStore<T>,
}

impl Node {
    /// Returns an iterator that yields the indices of the nodes that are
    /// reachable from `self`.
    pub fn entries(&self) -> slice::Iter<usize> {
        match self.entries.as_ref() {
            Some(entries) => entries.iter(),
            None => [].iter(),
        }
    }
}

impl Node {
    /// Pushes a new index into the entries of the node and returns the index.
    fn push(&mut self, index: usize) -> usize {
        self.entries.get_or_insert_with(Vec::new).push(index);
        index
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
    pub fn entry(&mut self, index: usize) -> RouteEntry<T> {
        RouteEntry {
            node_index: index,
            route_store: self,
        }
    }

    /// Returns a shared reference to the node at the given index.
    pub fn node(&self, index: usize) -> &Node {
        self.nodes.get(index).unwrap()
    }

    /// Returns a mutable reference to the node at the given index.
    pub fn node_mut(&mut self, index: usize) -> &mut Node {
        self.nodes.get_mut(index).unwrap()
    }

    /// Pushes a new node into the store and returns the index of the newly
    /// inserted node.
    pub fn push(&mut self, node: Node) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    /// Returns a mutable reference to the route at the given index.
    pub fn route_mut(&mut self, index: usize) -> &mut T {
        self.routes.get_mut(index).unwrap()
    }

    /// Returns an optional shared reference to the route of the node at the
    /// given index if it exists.
    pub fn route_at_node(&self, index: usize) -> Option<&T> {
        let route_index = self.route_index_at_node(index)?;
        Some(self.routes.get(route_index)?)
    }
}

impl<T> RouteStore<T> {
    /// Pushes a new route into the store and returns the index of the newly
    /// inserted route.
    fn push_route(&mut self, route: Box<T>) -> usize {
        let index = self.routes.len();
        self.routes.push(route);
        index
    }

    /// Returns the index of the route in the route store associated with the
    /// node at the given index if it exists.
    fn route_index_at_node(&self, index: usize) -> Option<usize> {
        self.node(index).route
    }
}

impl<'a, T> RouteEntry<'a, T> {
    /// Pushes a new node into the store and associates the index of the new
    /// node to the entries of the current node. Returns the index of the
    /// newly inserted node.
    pub fn push_node(&mut self, node: Node) -> usize {
        // Push the node into the store and get the index of the newly inserted
        // node.
        let next_node_index = self.route_store.push(node);

        // Associate the index of the newly inserted node with the current node
        // by adding the index to the entries of the current node.
        self.route_store
            .node_mut(self.node_index)
            .push(next_node_index);

        // Return the index of the newly inserted node in the store.
        next_node_index
    }

    /// Inserts a new route into the store and associates the index of the new
    /// route to the current node. Returns the index of the newly inserted route.
    pub fn insert_route(&mut self, route: Box<T>) -> usize {
        // Push the route into the store and get the index of the newly
        // inserted route.
        let route_index = self.route_store.push_route(route);

        // Associate the route index with the current node.
        self.route_store.node_mut(self.node_index).route = Some(route_index);

        // Return the index of the newly inserted route in the store.
        route_index
    }

    /// Returns the index of the route at the current node. If the node does not
    /// have a route associated with it, a new route will be inserted by calling
    /// the provided closure `f`.
    pub fn get_or_insert_route_with<F>(&mut self, f: F) -> usize
    where
        F: FnOnce() -> Box<T>,
    {
        self.route_store
            // Get the index of the route associated with the node if it exists.
            .route_index_at_node(self.node_index)
            // If the node does not have a route associated with it, insert a
            // new route into the store, associate it with the node, and
            // return the index of the route in the store.
            .unwrap_or_else(|| self.insert_route(f()))
    }
}
