use smallvec::SmallVec;
use std::{iter::Peekable, rc::Rc, str::CharIndices};

use crate::node::{Node, Pattern};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Match<'a, 'b, T> {
    pub is_exact: bool,
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
    path_segments: Rc<SmallVec<[(usize, &'b str); 4]>>,
    visitor_delegate: Option<Box<Self>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Segments<'a> {
    chars: Peekable<CharIndices<'a>>,
    value: &'a str,
}

impl<'a> Segments<'a> {
    pub(crate) fn new(value: &'a str) -> Self {
        Segments {
            chars: value.char_indices().peekable(),
            value,
        }
    }
}

impl Segments<'static> {
    pub(crate) fn patterns(self) -> impl Iterator<Item = Pattern> {
        self.map(|(_, value)| Pattern::from(value))
    }
}

impl<'a> Iterator for Segments<'a> {
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

impl<'a, 'b, T: Default> Visit<'a, 'b, T> {
    pub(crate) fn root(node: &'a Node<T>, path: &'b str) -> Self {
        let segments = Segments::new(path).collect();

        Visit {
            node,
            depth: 0,
            index: 0,
            path_value: path,
            path_segments: Rc::new(segments),
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

    fn delegate_next_match(&mut self) -> Option<Match<'a, 'b, T>> {
        self.visitor_delegate
            .as_mut()
            .and_then(|delegate| delegate.next())
            .or_else(|| {
                self.visitor_delegate = None;
                None
            })
    }

    fn find_next_match<F>(&mut self, mut predicate: F) -> Option<&'a Node<T>>
    where
        F: FnMut(&'a Node<T>) -> bool,
    {
        match self.node.find(self.index, &mut predicate) {
            Some((index, next)) => {
                self.index = index + 1;
                Some(next)
            }
            None => {
                self.index = self.node.entries.len();
                None
            }
        }
    }

    fn get_path_segment_value(&self) -> Option<&'b str> {
        self.path_segments.get(self.depth).map(|(_, value)| *value)
    }

    fn get_remaining_path(&self) -> Option<&'b str> {
        self.path_segments
            .get(self.depth)
            .map(|(start, _)| self.path_value[*start..].trim_start_matches('/'))
    }

    fn get_param_for_pattern(
        &self,
        pattern: Pattern,
        path_segment: &'b str,
    ) -> Option<(&'static str, &'b str)> {
        if let Pattern::CatchAll(key) = pattern {
            Some((key, self.get_remaining_path().unwrap_or("")))
        } else if let Pattern::Dynamic(key) = pattern {
            Some((key, path_segment))
        } else {
            None
        }
    }
}

impl<'a, 'b, T: Default> Iterator for Visit<'a, 'b, T> {
    type Item = Match<'a, 'b, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // First, delegate to the next visitor to see if there are any matches
        // from decedent nodes.
        if let Some(component) = self.delegate_next_match() {
            return Some(component);
        }

        if let Some(path_segment) = self.get_path_segment_value() {
            let mut is_exact = self.depth == self.path_segments.len() - 1;
            let next = self.find_next_match(|entry| path_segment == entry.pattern)?;

            if matches!(next.pattern, Pattern::CatchAll(_)) {
                is_exact = true;
            } else {
                self.visitor_delegate = Some(self.fork(next));
            }

            return Some(Match {
                is_exact,
                pattern: next.pattern,
                param: self.get_param_for_pattern(next.pattern, path_segment),
                route: &next.route,
            });
        }

        let next = self.find_next_match(|entry| matches!(entry.pattern, Pattern::CatchAll(_)))?;

        Some(Match {
            is_exact: true,
            pattern: next.pattern,
            param: self.get_param_for_pattern(next.pattern, ""),
            route: &next.route,
        })
    }
}
