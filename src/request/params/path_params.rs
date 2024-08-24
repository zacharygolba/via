use std::fmt::{self, Debug, Formatter};
use std::iter::Extend;

pub struct PathParams {
    data: Vec<(&'static str, (usize, usize))>,
}

impl PathParams {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn get(&self, predicate: &str) -> Option<(usize, usize)> {
        self.data.iter().find_map(
            |(name, at)| {
                if *name == predicate {
                    Some(*at)
                } else {
                    None
                }
            },
        )
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}

impl Extend<(&'static str, (usize, usize))> for PathParams {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (&'static str, (usize, usize))>,
    {
        self.data.extend(iter);
    }
}
