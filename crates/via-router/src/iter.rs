use std::{iter::Peekable, ops::Deref, rc::Rc, str::CharIndices};

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
    depth: usize,
    index: usize,
    path_value: &'b str,
    path_segments: Rc<Vec<(usize, &'b str)>>,
    visitor_delegate: Option<Box<Self>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Path<'a> {
    chars: Peekable<CharIndices<'a>>,
    value: &'a str,
}

impl<'a, 'b, T> Deref for Component<'a, 'b, T> {
    type Target = &'a T;

    fn deref(&self) -> &Self::Target {
        &self.route
    }
}

impl<'a> Path<'a> {
    pub fn new(value: &'a str) -> Path<'a> {
        Path {
            chars: value.char_indices().peekable(),
            value,
        }
    }
}

impl Path<'static> {
    pub fn segments(source: &'static str) -> impl Iterator<Item = Pattern> {
        Path::new(source).map(|(_, segment)| segment.into())
    }
}

impl<'a> Iterator for Path<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = None;
        let mut end = self.value.len();

        while let (index, '/') = *self.chars.peek()? {
            start = Some(index);
            self.chars.next();
        }

        while let Some((index, value)) = self.chars.peek() {
            if *value == '/' {
                end = *index;
                break;
            }

            self.chars.next();
        }

        Some((start?, &self.value[(start? + 1)..end]))
    }
}

impl<'a> PartialEq<&'_ str> for Path<'a> {
    fn eq(&self, other: &'_ &str) -> bool {
        self.value == *other
    }
}

impl<'a, 'b, T: Default> Visit<'a, 'b, T> {
    pub(crate) fn root(node: &'a Node<T>, path: &'b str) -> Self {
        let path_segments = Path::new(path).collect();

        Visit {
            node,
            depth: 0,
            index: 0,
            path_value: path,
            path_segments: Rc::new(path_segments),
            visitor_delegate: None,
        }
    }

    fn fork(&self, node: &'a Node<T>) -> Box<Self> {
        Box::new(Visit {
            node,
            index: 0,
            depth: self.depth + 1,
            path_value: self.path_value,
            path_segments: Rc::clone(&self.path_segments),
            visitor_delegate: None,
        })
    }

    fn is_last(&self) -> bool {
        self.depth == self.path_segments.len() - 1
    }

    fn delegate_next(&mut self) -> Option<Component<'a, 'b, T>> {
        self.visitor_delegate
            .as_mut()
            .and_then(|delegate| delegate.next())
            .or_else(|| {
                self.visitor_delegate = None;
                None
            })
    }

    fn get_path_segment_value(&self) -> Option<&'b str> {
        self.path_segments.get(self.depth).map(|(_, value)| *value)
    }

    fn get_remaining_path(&self) -> Option<&'b str> {
        self.path_segments
            .get(self.depth)
            .map(|(start, _)| &self.path_value[*start..])
    }
}

impl<'a, 'b, T: Default> Iterator for Visit<'a, 'b, T> {
    type Item = Component<'a, 'b, T>;

    fn next(&mut self) -> Option<Self::Item> {
        println!(
            "Visit::next pattern = {:?}, depth = {}, index = {}",
            self.node.pattern, self.depth, self.index
        );

        // First, delegate to the next visitor to see if there are any matches
        // from decedent nodes.
        if let Some(component) = self.delegate_next() {
            return Some(component);
        }

        if self.get_path_segment_value().is_none() {
            let (index, next) = self.node.find(self.index, |entry| {
                matches!(entry.pattern, Pattern::Root | Pattern::CatchAll(_))
            })?;

            self.index = index + 1;

            return Some(Component {
                is_exact_match: true,
                pattern: next.pattern,
                param: Some((next.pattern.name().unwrap(), "")),
                route: &next.route,
            });
        }

        let path_segment = self.get_path_segment_value()?;
        let (index, next) = self
            .node
            .find(self.index, |entry| entry.pattern == *path_segment)?;
        let is_catch_all = matches!(next.pattern, Pattern::CatchAll(_));

        self.index = index + 1;

        if !is_catch_all {
            self.visitor_delegate = Some(self.fork(next));
        }

        Some(Component {
            is_exact_match: is_catch_all || self.is_last(),
            pattern: next.pattern,
            param: match next.pattern {
                Pattern::CatchAll(param_name) => Some((param_name, self.get_remaining_path()?)),
                Pattern::Dynamic(param_name) => Some((param_name, path_segment)),
                _ => None,
            },
            route: &next.route,
        })
    }
}
