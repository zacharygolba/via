use std::iter::Copied;
use std::slice;

use smallvec::SmallVec;

use crate::path::Pattern;
use crate::Param;

/// A node in the route tree that represents a single path segment.
pub struct Node<T> {
    /// The index of the route in the route store associated with the node.
    pub route: Option<T>,

    /// The pattern used to match the node against a path segment.
    pub pattern: Pattern,

    /// The indices of the nodes that are reachable from the current node.
    children: SmallVec<[usize; 2]>,
}

impl<T> Node<T> {
    pub fn new(pattern: Pattern) -> Self {
        Self {
            children: SmallVec::new(),
            route: None,
            pattern,
        }
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

    pub fn children(&self) -> Copied<slice::Iter<usize>> {
        self.children.iter().copied()
    }
}

impl<T> Node<T> {
    /// Pushes a new key into the entries of the node and return it.
    pub(crate) fn push(&mut self, key: usize) -> usize {
        self.children.push(key);
        key
    }
}
