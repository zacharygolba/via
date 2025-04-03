use smallvec::SmallVec;
use std::slice;

use crate::router::Node;

#[derive(Debug, PartialEq)]
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
    range: Option<[usize; 2]>,
    nodes: SmallVec<[MatchKind<'a, T>; 1]>,
}

impl<T> Binding<'_, T> {
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[inline]
    pub fn nodes(&self) -> slice::Iter<MatchKind<T>> {
        self.nodes.iter()
    }

    #[inline]
    pub fn range(&self) -> Option<&[usize; 2]> {
        self.range.as_ref()
    }
}

impl<'a, T> Binding<'a, T> {
    #[inline]
    pub(crate) fn new(range: Option<[usize; 2]>, nodes: SmallVec<[MatchKind<'a, T>; 1]>) -> Self {
        Self { range, nodes }
    }

    #[inline]
    pub(crate) fn push(&mut self, node: MatchKind<'a, T>) {
        self.nodes.push(node);
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
