use std::iter::Peekable;
use std::str::CharIndices;

use super::{query_parser, Param};

pub struct QueryParam<'a, 'b> {
    name: &'b str,
    query: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

impl QueryParam<'_, '_> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Param {
        Param::new(self.find_next(), self.name, self.query)
    }

    pub fn last(&mut self) -> Param {
        let mut at = None;

        while let Some(next) = self.find_next() {
            at = Some(next);
        }

        Param::new(at, self.name, self.query)
    }

    pub fn get(&mut self, index: usize) -> Param {
        let mut at = None;

        for _ in 0..index {
            at = self.find_next();
        }

        Param::new(at, self.name, self.query)
    }
}

impl<'a, 'b> QueryParam<'a, 'b> {
    pub(crate) fn new(name: &'b str, query: &'a str) -> Self {
        let chars = query.char_indices().peekable();

        Self { name, query, chars }
    }

    fn find_next(&mut self) -> Option<(usize, usize)> {
        let chars = &mut self.chars;
        let query = self.query;

        loop {
            let (name, at) = query_parser::parse(chars, query)?;

            if name == self.name {
                return Some(at);
            }
        }
    }
}
