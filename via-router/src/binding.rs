use smallvec::SmallVec;

use crate::router::Node;

/// A group of nodes that match the path segment at `self.range`.
///
#[derive(Debug)]
pub struct Binding<'a, T> {
    is_final: bool,
    nodes: SmallVec<[&'a Node<T>; 1]>,
    range: Option<[usize; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MatchCond<T> {
    Partial(T),
    Final(T),
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
    pub fn results(&self) -> impl Iterator<Item = &Node<T>> {
        self.nodes.iter().map(|node| *node)
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
    pub(crate) fn push(&mut self, node: &'a Node<T>) {
        self.nodes.push(node);
    }
}
