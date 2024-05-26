use smallvec::SmallVec;
use std::cmp::{Ordering, PartialOrd};

use crate::iter::Labels;

#[derive(Clone, Debug)]
pub struct Node<T> {
    pub(crate) children: SmallVec<[Box<Self>; 4]>,
    pub(crate) pattern: Pattern,
    pub(crate) route: T,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq)]
pub enum Pattern {
    CatchAll(&'static str),
    Dynamic(&'static str),
    Static(&'static str),
    Root,
}

impl<T: Default> Node<T> {
    pub fn find(&self, label: &str) -> Option<&Self> {
        self.children.iter().find_map(|child| {
            if child.pattern == *label {
                Some(&**child)
            } else {
                None
            }
        })
    }

    pub fn index(&self, label: Pattern) -> Option<usize> {
        self.children
            .iter()
            .position(|child| label == child.pattern)
    }

    pub fn insert(&mut self, path: &mut Labels) -> &mut Self {
        if let Pattern::CatchAll(_) = self.pattern {
            return self;
        }

        let label = match path.next() {
            Some(value) => value,
            None => return self,
        };

        let index = match self.index(label) {
            Some(value) => value,
            None => insert1(self, label),
        };

        self.children[index].insert(path)
    }
}

impl<T: Default> Default for Node<T> {
    fn default() -> Node<T> {
        Node {
            children: SmallVec::new(),
            pattern: Pattern::Root,
            route: Default::default(),
        }
    }
}

impl From<&'static str> for Pattern {
    fn from(value: &'static str) -> Pattern {
        match value.chars().next() {
            Some('*') => Pattern::CatchAll(&value[1..]),
            Some(':') => Pattern::Dynamic(&value[1..]),
            _ => Pattern::Static(value),
        }
    }
}

impl PartialEq<str> for Pattern {
    fn eq(&self, other: &str) -> bool {
        if let Pattern::Static(value) = *self {
            value == other
        } else {
            true
        }
    }
}

impl PartialOrd for Pattern {
    fn partial_cmp(&self, other: &Pattern) -> Option<Ordering> {
        Some(match self {
            Pattern::CatchAll(_) => match other {
                Pattern::CatchAll(_) | Pattern::Root => Ordering::Equal,
                _ => Ordering::Greater,
            },
            Pattern::Dynamic(_) => match other {
                Pattern::CatchAll(_) | Pattern::Root => Ordering::Less,
                Pattern::Dynamic(_) => Ordering::Equal,
                Pattern::Static(_) => Ordering::Greater,
            },
            Pattern::Static(a) => match other {
                Pattern::Static(b) => a.partial_cmp(b)?,
                _ => Ordering::Less,
            },
            Pattern::Root => match other {
                Pattern::CatchAll(_) | Pattern::Root => Ordering::Equal,
                _ => Ordering::Greater,
            },
        })
    }
}

fn insert1<T: Default>(parent: &mut Node<T>, pattern: Pattern) -> usize {
    let mut offset = 0;
    let child = Node {
        pattern,
        ..Default::default()
    };

    for (index, other) in parent.children.iter().enumerate() {
        offset = match other.pattern {
            Pattern::Static(_) => index + 1,
            _ => index,
        };

        if child.pattern < other.pattern {
            break;
        }
    }

    parent.children.insert(offset, Box::new(child));
    offset
}
