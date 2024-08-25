use core::slice;

use crate::path::Pattern;

/// A node in the route tree that represents a single path segment.
pub struct Node<T> {
    /// The indices of the nodes that are reachable from the current node.
    pub entries: Option<Vec<usize>>,

    /// The pattern used to match the node against a path segment.
    pub pattern: Pattern,

    /// The index of the route in the route store associated with the node.
    route: Option<T>,
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
        self.route.as_ref()
    }

    /// Returns a mutable reference to the `route` field of the node.
    pub fn route_mut(&mut self) -> &mut Option<T> {
        &mut self.route
    }
}

impl<T> Node<T> {
    /// Pushes a new key into the entries of the node and return it.
    fn push(&mut self, key: usize) -> usize {
        self.entries.get_or_insert_with(Vec::new).push(key);
        key
    }
}

/// Like `Option::expect`, but in B minor.
macro_rules! unwrap_node {
    (
        // The option that contains the `&Node<_>` or `&mut Node<_>` that we'll
        // attempt to unwrap.
        $option:expr,
        // The key that was used to look up the node contained in `$option` from
        // the route store.
        $at:expr
    ) => {
        match $option {
            Some(node) => node,
            None => {
                // This should never happen.
                //
                // The router is like the Hotel California. You can check out any
                // time you like, but you can never leave.
                //
                // If you see this error, you've probably found a bug in the way
                // that routes are added to the route store or the way the key
                // is assigned to nodes.
                if cfg!(debug_assertions) {
                    panic!("unknown key: {}", $at);
                }

                panic!("unknown key");
            }
        }
    };
}

impl<'a, T> RouteEntry<'a, T> {
    /// Pushes a new node into the store and associates the index of the new
    /// node to the entries of the current node. Returns the index of the
    /// newly inserted node.
    pub fn push(&mut self, node: Node<T>) -> usize {
        // Push the node into the store and get the key of the newly inserted
        // node.
        let next_node_key = self.store.push(node);

        // Associate the key of the newly inserted node with the current node
        // by adding the key to the entries of the current node.
        self.store.get_mut(self.key).push(next_node_key)
    }
}

impl<T> RouteStore<T> {
    /// Constructs a new, empty `RouteStore`.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Returns a mutable representation of a single node in the route store.
    pub fn entry(&mut self, key: usize) -> RouteEntry<T> {
        RouteEntry { key, store: self }
    }

    /// Pushes a new node into the store and returns the key of the newly
    /// inserted node.
    pub fn push(&mut self, node: Node<T>) -> usize {
        let key = self.nodes.len();

        self.nodes.push(node);
        key
    }

    /// Shrinks the capacity of the route store as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.nodes.shrink_to_fit();
    }

    /// Returns a shared reference to the node at the given `key`.
    pub fn get(&self, key: usize) -> &Node<T> {
        unwrap_node!(self.nodes.get(key), key)
    }

    /// Returns a mutable reference to the node at the given `key`.
    pub fn get_mut(&mut self, key: usize) -> &mut Node<T> {
        unwrap_node!(self.nodes.get_mut(key), key)
    }
}
