use smallvec::SmallVec;
use std::slice;

use crate::router::Node;

#[derive(Clone, Debug, PartialEq)]
pub enum MatchCond<T> {
    Partial(T),
    Exact(T),
}

#[derive(Debug)]
pub enum MatchKind<'a, T> {
    Edge(MatchCond<&'a Node<T>>),
    Wildcard(&'a Node<T>),
}

/// A group of nodes that match the path segment at `self.range`.
///
#[derive(Debug)]
pub struct Binding<'a, T> {
    has_nodes: bool,
    nodes: SmallVec<[MatchKind<'a, T>; 1]>,
    range: Option<[usize; 2]>,
}

impl<'a, T> Binding<'a, T> {
    #[inline]
    pub fn has_nodes(&self) -> bool {
        self.has_nodes
    }

    #[inline]
    pub fn nodes(&self) -> slice::Iter<'_, MatchKind<'a, T>> {
        self.nodes.iter()
    }

    #[inline]
    pub fn range(&self) -> Option<[usize; 2]> {
        self.range
    }
}

impl<'a, T> Binding<'a, T> {
    #[inline]
    pub(crate) fn new(range: [usize; 2]) -> Self {
        Self {
            has_nodes: false,
            nodes: SmallVec::new(),
            range: Some(range),
        }
    }

    pub(crate) fn new_with_nodes(
        range: Option<[usize; 2]>,
        nodes: SmallVec<[MatchKind<'a, T>; 1]>,
    ) -> Self {
        debug_assert!(
            !nodes.is_empty(),
            "Binding::new_with_nodes requires that nodes is not empty"
        );

        Self {
            has_nodes: true,
            nodes,
            range,
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, node: MatchKind<'a, T>) {
        self.nodes.push(node);
        self.has_nodes = true;
    }
}

impl<T> MatchCond<T> {
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> MatchCond<U> {
        match self {
            Self::Partial(input) => MatchCond::Partial(f(input)),
            Self::Exact(input) => MatchCond::Exact(f(input)),
        }
    }

    #[inline]
    pub fn matches<U>(&self, other: MatchCond<U>) -> Option<U> {
        match (self, other) {
            (Self::Partial(_), MatchCond::Partial(value))
            | (Self::Exact(_), MatchCond::Exact(value)) => Some(value),
            _ => None,
        }
    }

    #[inline]
    pub fn as_either(&self) -> &T {
        match self {
            Self::Exact(value) | Self::Partial(value) => value,
        }
    }

    #[inline]
    pub fn as_partial(&self) -> Option<&T> {
        if let Self::Partial(value) = self {
            Some(value)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_ref(&self) -> MatchCond<&T> {
        match *self {
            Self::Exact(ref value) => MatchCond::Exact(value),
            Self::Partial(ref value) => MatchCond::Partial(value),
        }
    }

    #[inline]
    pub fn as_mut(&mut self) -> MatchCond<&mut T> {
        match *self {
            Self::Exact(ref mut value) => MatchCond::Exact(value),
            Self::Partial(ref mut value) => MatchCond::Partial(value),
        }
    }
}

impl<'a, T> MatchKind<'a, T> {
    #[inline]
    pub(crate) fn edge(is_exact: bool, node: &'a Node<T>) -> Self {
        Self::Edge(if is_exact {
            MatchCond::Exact(node)
        } else {
            MatchCond::Partial(node)
        })
    }

    #[inline]
    pub(crate) fn wildcard(node: &'a Node<T>) -> Self {
        Self::Wildcard(node)
    }
}
