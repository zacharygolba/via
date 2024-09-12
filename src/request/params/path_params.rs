use std::fmt::{self, Debug, Formatter};

pub struct PathParams {
    data: Vec<(&'static str, (usize, usize))>,
}

impl PathParams {
    pub fn new() -> Self {
        Self { data: Vec::new() }
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

    pub fn push(&mut self, param: (&'static str, (usize, usize))) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}

impl Default for PathParams {
    fn default() -> Self {
        Self::new()
    }
}
