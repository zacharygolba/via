use smallvec::SmallVec;
use std::cmp::{Ordering, PartialOrd};

#[derive(Clone, Debug)]
pub struct Node<T> {
    pub(crate) entries: SmallVec<[Box<Self>; 4]>,
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
    pub fn find(&self, path: &str) -> Option<&Self> {
        self.entries.iter().find_map(|node| {
            if node.pattern == *path {
                Some(&**node)
            } else {
                None
            }
        })
    }

    pub fn index(&self, pattern: Pattern) -> Option<usize> {
        self.entries.iter().position(|node| pattern == node.pattern)
    }

    pub fn insert<I>(&mut self, segments: &mut I) -> &mut Self
    where
        I: Iterator<Item = Pattern>,
    {
        if let Pattern::CatchAll(_) = self.pattern {
            return self;
        }

        let label = match segments.next() {
            Some(value) => value,
            None => return self,
        };

        let index = match self.index(label) {
            Some(value) => value,
            None => insert1(self, label),
        };

        self.entries[index].insert(segments)
    }
}

impl<T: Default> Default for Node<T> {
    fn default() -> Node<T> {
        Node {
            entries: SmallVec::new(),
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

fn insert1<T: Default>(node: &mut Node<T>, pattern: Pattern) -> usize {
    let mut offset = 0;

    for (index, entry) in node.entries.iter().enumerate() {
        offset = match entry.pattern {
            Pattern::Static(_) => index + 1,
            _ => index,
        };

        if pattern < entry.pattern {
            break;
        }
    }

    node.entries.insert(
        offset,
        Box::new(Node {
            pattern,
            ..Default::default()
        }),
    );

    offset
}
