#[derive(Clone, Debug)]
pub struct Node<T> {
    pub(crate) entries: Vec<Box<Self>>,
    pub(crate) pattern: Pattern,
    pub(crate) route: T,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pattern {
    CatchAll(&'static str),
    Dynamic(&'static str),
    Static(&'static str),
    Root,
}

impl<T: Default> Node<T> {
    pub(crate) fn new(pattern: Pattern) -> Self {
        Node {
            pattern,
            entries: Vec::new(),
            route: Default::default(),
        }
    }

    pub(crate) fn find<'a, F>(
        &'a self,
        from_index: usize,
        mut predicate: F,
    ) -> Option<(usize, &'a Node<T>)>
    where
        F: FnMut(&'a Node<T>) -> bool,
    {
        self.entries
            .iter()
            .skip(from_index)
            .enumerate()
            .find_map(|(index, node)| {
                if predicate(node) {
                    Some((from_index + index, &**node))
                } else {
                    None
                }
            })
    }

    pub(crate) fn insert<I>(&mut self, segments: &mut I) -> &mut Self
    where
        I: Iterator<Item = Pattern>,
    {
        if let Pattern::CatchAll(_) = self.pattern {
            while let Some(_) = segments.next() {}
            return self;
        }

        let pattern = match segments.next() {
            Some(value) => value,
            None => return self,
        };

        if let Some(index) = self.entries.iter().position(|node| pattern == node.pattern) {
            self.entries[index].insert(segments)
        } else {
            let index = self.entries.len();
            let entry = Node::new(pattern);

            self.entries.push(Box::new(entry));
            self.entries[index].insert(segments)
        }
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

impl From<&'static str> for Pattern {
    fn from(value: &'static str) -> Pattern {
        match value.chars().next() {
            Some('*') => Pattern::CatchAll(&value[1..]),
            Some(':') => Pattern::Dynamic(&value[1..]),
            _ => Pattern::Static(value),
        }
    }
}

impl PartialEq<&str> for Pattern {
    fn eq(&self, other: &&str) -> bool {
        if let Pattern::Static(value) = *self {
            value == *other
        } else {
            true
        }
    }
}

impl PartialEq<Pattern> for &str {
    fn eq(&self, other: &Pattern) -> bool {
        other == self
    }
}
