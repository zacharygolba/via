use smallvec::SmallVec;
use std::slice;

use crate::path::{Param, Pattern};

/// A group of nodes that match the path segment at `self.range`.
///
#[derive(Debug)]
pub struct Binding<'a, T> {
    is_final: bool,
    nodes: SmallVec<[Match<'a, T>; 1]>,
    range: Option<[usize; 2]>,
}

#[derive(Debug)]
pub struct Match<'a, T> {
    pattern: &'a Pattern,
    route: &'a [MatchCond<T>],
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MatchCond<T> {
    Partial(T),
    Exact(T),
}

impl<'a, T> Binding<'a, T> {
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[inline]
    pub fn range(&self) -> Option<&[usize; 2]> {
        self.range.as_ref()
    }

    #[inline]
    pub fn results(&self) -> slice::Iter<'_, Match<'a, T>> {
        self.nodes.iter()
    }

    #[inline]
    pub fn is_final(&self) -> bool {
        self.is_final
    }
}

impl<'a, T> Binding<'a, T> {
    #[inline]
    pub(crate) fn new(is_final: bool, range: Option<[usize; 2]>) -> Self {
        Self {
            is_final,
            range,
            nodes: SmallVec::new(),
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, node: Match<'a, T>) {
        self.nodes.push(node);
    }
}

impl<'a, T> Match<'a, T> {
    #[inline]
    pub(crate) fn new(pattern: &'a Pattern, route: &'a [MatchCond<T>]) -> Self {
        Self { pattern, route }
    }
}

impl<'a, T> Match<'a, T> {
    #[inline]
    pub fn is_wildcard(&self) -> bool {
        matches!(self.pattern, Pattern::Wildcard(_))
    }

    #[inline]
    pub fn param(&self) -> Option<&Param> {
        if let Pattern::Dynamic(param) | Pattern::Wildcard(param) = self.pattern {
            Some(param)
        } else {
            None
        }
    }

    pub fn exact(&self) -> impl Iterator<Item = &T> {
        self.route.iter().map(|cond| match cond {
            MatchCond::Partial(partial) => partial,
            MatchCond::Exact(exact) => exact,
        })
    }

    pub fn partial(&self) -> impl Iterator<Item = &T> {
        self.route.iter().filter_map(|cond| match cond {
            MatchCond::Partial(partial) => Some(partial),
            MatchCond::Exact(_) => None,
        })
    }
}
