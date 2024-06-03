use std::{
    cmp::{Ordering, PartialOrd},
    slice,
};

#[derive(Clone, Debug)]
pub struct Node<T> {
    pub(crate) entries: Vec<Box<Self>>,
    pub(crate) pattern: Pattern,
    pub(crate) route: T,
}

#[derive(Debug)]
pub struct Visitor<'a, 'b, T> {
    entries: slice::Iter<'a, Box<Node<T>>>,
    predicate: &'b str,
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
    fn new(pattern: Pattern) -> Node<T> {
        Node {
            pattern,
            entries: Vec::new(),
            route: Default::default(),
        }
    }

    pub fn find<F>(&self, offset: usize, mut predicate: F) -> Option<(usize, &Node<T>)>
    where
        F: FnMut(&Node<T>) -> bool,
    {
        println!("    find from offset: {}", offset);

        self.entries
            .iter()
            .skip(offset)
            .enumerate()
            .find_map(|(index, node)| {
                if predicate(node) {
                    println!("        index: {}, node: {:?}", index, node.pattern);
                    Some((offset + index, &**node))
                } else {
                    None
                }
            })
    }

    pub fn index(&self, pattern: Pattern) -> Option<usize> {
        self.entries.iter().position(|node| pattern == node.pattern)
    }

    // pub fn insert<I>(&mut self, segments: &mut I) -> &mut Self
    // where
    //     I: Iterator<Item = Pattern>,
    // {
    //     if let Pattern::CatchAll(_) = self.pattern {
    //         return self;
    //     }

    //     let pattern = match segments.next() {
    //         Some(value) => value,
    //         None => return self,
    //     };

    //     if let Some(index) = self.index(pattern) {
    //         self.entries[index].insert(segments)
    //     } else {
    //         let index = self.entries.len();
    //         let entry = Node::new(pattern);

    //         self.entries.push(Box::new(entry));
    //         &mut self.entries[index]
    //     }
    // }

    pub fn insert<I>(&mut self, segments: &mut I) -> &mut Self
    where
        I: Iterator<Item = Pattern>,
    {
        if let Pattern::CatchAll(_) = self.pattern {
            return self;
        }

        let pattern = match segments.next() {
            Some(value) => value,
            None => return self,
        };

        let index = match self.index(pattern) {
            Some(value) => value,
            None => insert1(self, pattern),
        };

        self.entries[index].insert(segments)
    }
}

impl<T: Default> Default for Node<T> {
    fn default() -> Node<T> {
        Node {
            entries: Vec::new(),
            pattern: Pattern::Root,
            route: Default::default(),
        }
    }
}

impl Pattern {
    pub fn name(&self) -> Option<&'static str> {
        match self {
            Pattern::CatchAll(name) | Pattern::Dynamic(name) => Some(name),
            _ => None,
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

impl<'a, 'b, T> Iterator for Visitor<'a, 'b, T> {
    type Item = &'a Node<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.entries.next()?;

        if next.pattern == *self.predicate {
            Some(next)
        } else {
            None
        }
    }
}

fn insert1<T: Default>(node: &mut Node<T>, pattern: Pattern) -> usize {
    // let mut offset = 0;

    // for (index, entry) in node.entries.iter().enumerate() {
    //     offset = match entry.pattern {
    //         Pattern::Static(_) => index + 1,
    //         _ => index,
    //     };

    //     if pattern < entry.pattern {
    //         break;
    //     }
    // }

    let offset = node.entries.len();
    node.entries.push(Box::new(Node {
        pattern,
        ..Default::default()
    }));

    assert!(node.entries[offset].pattern == pattern);

    // node.entries.insert(
    //     offset,
    //     Box::new(Node {
    //         pattern,
    //         ..Default::default()
    //     }),
    // );

    offset
}
