use std::{iter::Peekable, ops::Deref, str::CharIndices};

use crate::node::{Node, Pattern};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Component<'a, 'b, T> {
    pub exact: bool,
    pub label: Pattern,
    pub param: Option<(&'static str, &'b str)>,
    pub route: &'a T,
}

#[derive(Debug)]
pub struct Labels {
    path: Path<'static>,
}

#[derive(Debug)]
pub struct Path<'a> {
    iter: Peekable<CharIndices<'a>>,
    next: Option<(usize, &'a str)>,
    path: &'a str,
}

#[derive(Debug)]
pub struct Visit<'a, 'b, T> {
    pub(crate) node: &'a Node<T>,
    pub(crate) path: Path<'b>,
    pub(crate) root: bool,
}

impl<'a, 'b, T> Component<'a, 'b, T> {
    pub(crate) fn root(route: &'a T, exact: bool) -> Component<'a, 'b, T> {
        Component {
            label: Pattern::Root,
            param: None,
            exact,
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

impl Labels {
    pub fn parse(path: &'static str) -> Labels {
        Labels {
            path: Path::parse(path),
        }
    }
}

impl Iterator for Labels {
    type Item = Pattern;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.path.next().map(|(_, value)| value)?.into())
    }
}

impl<'a> Path<'a> {
    pub fn parse(path: &'a str) -> Path<'a> {
        Path {
            iter: path.char_indices().peekable(),
            next: None,
            path,
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
        &self.path[from..]
    }

    fn advance(&mut self) -> Option<(usize, &'a str)> {
        let mut start = None;
        let mut end = self.path.len();

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

        Some((start?, &self.path[(start? + 1)..end]))
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
        self.path == *other
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
        let (start, value) = path.next()?;
        let edge = node.find(value)?;

        *node = edge;

        Some(Component {
            exact: path.peek().is_none(),
            label: edge.pattern,
            param: match edge.pattern {
                Pattern::CatchAll(name) => Some((name, path.slice(start))),
                Pattern::Dynamic(name) => Some((name, value)),
                _ => None,
            },
            route: &edge.route,
        })
    }
}
