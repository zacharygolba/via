use smallvec::SmallVec;
use std::slice;

use crate::path::Param;

pub struct Binding<'a, T> {
    pub range: Option<[usize; 2]>,
    nodes: SmallVec<[Match<'a, T>; 1]>,
}

pub struct Iter<'a, T> {
    is_exact: bool,
    route: slice::Iter<'a, MatchCond<T>>,
}

pub struct Match<'a, T> {
    pub is_exact: bool,
    pub param: Option<&'a Param>,

    pub(crate) route: &'a [MatchCond<T>],
}

#[derive(Clone, Debug)]
pub(crate) enum MatchCond<T> {
    Partial(T),
    Exact(T),
}

impl<T> Binding<'_, T> {
    pub fn iter(&self) -> slice::Iter<Match<T>> {
        self.nodes.iter()
    }

    pub fn range(&self) -> Option<[usize; 2]> {
        self.range
    }
}

impl<'a, T> Binding<'a, T> {
    #[inline]
    pub(crate) fn new(range: Option<[usize; 2]>, nodes: SmallVec<[Match<'a, T>; 1]>) -> Self {
        Self { range, nodes }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[inline]
    pub(crate) fn push(&mut self, node: Match<'a, T>) {
        self.nodes.push(node);
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let is_exact = self.is_exact;

        self.route.find_map(|cond| match cond {
            MatchCond::Partial(partial) => Some(partial),

            MatchCond::Exact(exact) if is_exact => Some(exact),
            MatchCond::Exact(_) => None,
        })
    }
}

impl<'a, T> Match<'a, T> {
    #[inline]
    pub(crate) fn new(is_exact: bool, param: Option<&'a Param>, route: &'a [MatchCond<T>]) -> Self {
        Self {
            is_exact,
            param,
            route,
        }
    }

    #[inline]
    pub fn iter(&self) -> Iter<'a, T> {
        Iter {
            is_exact: self.is_exact,
            route: self.route.iter(),
        }
    }
}
