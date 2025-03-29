use std::slice;

use crate::path::Param;

pub struct Binding<'a, T> {
    range: Option<[usize; 2]>,
    nodes: Vec<Match<'a, T>>,
}

pub struct Match<'a, T> {
    exact: bool,
    param: Option<&'a Param>,
    route: &'a [MatchCond<T>],
}

pub struct Iter<'a, T> {
    exact: bool,
    route: slice::Iter<'a, MatchCond<T>>,
}

#[derive(Clone, Debug)]
pub enum MatchCond<T> {
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
    pub(crate) fn new(range: Option<[usize; 2]>, nodes: Vec<Match<'a, T>>) -> Self {
        Self { range, nodes }
    }
}

impl<T> Match<'_, T> {
    pub fn iter(&self) -> Iter<T> {
        Iter {
            exact: self.exact,
            route: self.route.iter(),
        }
    }

    pub fn param(&self) -> Option<&Param> {
        self.param
    }
}

impl<'a, T> Match<'a, T> {
    #[inline]
    pub(crate) fn new(exact: bool, param: Option<&'a Param>, route: &'a [MatchCond<T>]) -> Self {
        Self {
            exact,
            param,
            route,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.route.next()? {
                MatchCond::Partial(partial) => Some(partial),
                MatchCond::Exact(exact) => {
                    if !self.exact {
                        continue;
                    }

                    Some(exact)
                }
            };
        }
    }
}
