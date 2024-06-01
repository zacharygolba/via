use std::{iter::Peekable, ops::Deref, str::CharIndices};

use crate::node::{Node, Pattern};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Component<'a, 'b, T> {
    pub is_exact_match: bool,
    pub pattern: Pattern,
    pub param: Option<(&'static str, &'b str)>,
    pub route: &'a T,
}

#[derive(Debug)]
pub struct Visit<'a, 'b, T> {
    node: &'a Node<T>,
    path: Path<'b>,
    root: bool,
}

#[derive(Debug)]
pub(crate) struct Path<'a> {
    iter: Peekable<CharIndices<'a>>,
    next: Option<(usize, &'a str)>,
    value: &'a str,
}

impl<'a, 'b, T> Component<'a, 'b, T> {
    pub(crate) fn root(route: &'a T, is_exact_match: bool) -> Component<'a, 'b, T> {
        Component {
            pattern: Pattern::Root,
            param: None,
            is_exact_match,
            route,
        }
    }
}

impl<'a, 'b, T> Deref for Component<'a, 'b, T> {
    type Target = &'a T;

    fn deref(&self) -> &Self::Target {
        &self.route
    }
}

impl<'a> Path<'a> {
    pub fn parse(value: &'a str) -> Path<'a> {
        Path {
            iter: value.char_indices().peekable(),
            next: None,
            value,
        }
    }

    pub fn peek(&mut self) -> Option<(usize, &'a str)> {
        match self.next {
            next @ Some(_) => next,
            None => {
                self.next = self.advance();
                self.next
            }
        }
    }

    pub fn slice(&self, from: usize) -> &'a str {
        &self.value[from..]
    }

    fn advance(&mut self) -> Option<(usize, &'a str)> {
        let mut start = None;
        let mut end = self.value.len();

        while let (index, '/') = *self.iter.peek()? {
            start = Some(index);
            self.iter.next();
        }

        while let Some((index, value)) = self.iter.peek() {
            if *value == '/' {
                end = *index;
                break;
            }

            self.iter.next();
        }

        Some((start?, &self.value[(start? + 1)..end]))
    }
}

impl Path<'static> {
    pub fn segments(source: &'static str) -> impl Iterator<Item = Pattern> {
        Path::parse(source).map(|(_, segment)| segment.into())
    }
}

impl<'a> Iterator for Path<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().or_else(|| self.advance())
    }
}

impl<'a> PartialEq<&'_ str> for Path<'a> {
    fn eq(&self, other: &'_ &str) -> bool {
        self.value == *other
    }
}

impl<'a, 'b, T: Default> Visit<'a, 'b, T> {
    pub fn root(node: &'a Node<T>, path: &'b str) -> Self {
        Visit {
            node,
            path: Path::parse(path),
            root: true,
        }
    }
}

impl<'a, 'b, T: Default> Iterator for Visit<'a, 'b, T> {
    type Item = Component<'a, 'b, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.root {
            self.root = false;
            return Some(Component::root(&self.node.route, self.path == "/"));
        }

        let Visit { node, path, .. } = self;
        // Rather than returning early if there are no more path segments, we
        // must provide a default value to support resolving immediate children
        // of the root node.
        let (start, value) = path.next().unwrap_or((0, ""));
        let next = node.find(value)?;

        *node = next;

        Some(Component {
            is_exact_match: path.peek().is_none(),
            pattern: next.pattern,
            param: match next.pattern {
                Pattern::CatchAll(name) => Some((name, path.slice(start))),
                Pattern::Dynamic(name) => Some((name, value)),
                _ => None,
            },
            route: &next.route,
        })
    }
}
