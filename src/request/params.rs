use percent_encoding::percent_decode_str;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::{raise, Error};

#[derive(Debug, Default)]
pub struct Params {
    entries: HashMap<Box<str>, [usize; 2]>,
}

impl Params {
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(10),
        }
    }

    #[inline]
    pub fn get<'a>(&self, path: &'a str, name: &str) -> Result<Cow<'a, str>, Error> {
        self.entries
            .get(name)
            .and_then(|range| path.get(range[0]..range[1]))
            .map_or_else(
                || raise!(400, "missing required parameter '{}'", name),
                |value| {
                    percent_decode_str(value)
                        .decode_utf8()
                        .map_err(|e| Error::bad_request(e.into()))
                },
            )
    }
}

impl Params {
    #[inline]
    pub(crate) fn insert(&mut self, name: &Box<str>, range: [usize; 2]) {
        self.entries.insert(name.clone(), range);
    }
}
