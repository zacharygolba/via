use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::RwLock;

use crate::router::{Match, Matched};

#[derive(Debug)]
pub struct CacheError;

pub struct Cache {
    capacity: usize,
    entries: RwLock<VecDeque<(Box<str>, Matched)>>,
}

impl Error for CacheError {}

impl Display for CacheError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "lock in use")
    }
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: RwLock::new(VecDeque::with_capacity(capacity)),
        }
    }

    pub fn promote(&self, key: usize) {
        if let Ok(mut guard) = self.entries.try_write() {
            if self.capacity.checked_div(2).is_some_and(|half| key > half) {
                guard.swap_remove_front(key);
            }
        }
    }

    pub fn read(&self, path: &str) -> Result<Option<(usize, Matched)>, CacheError> {
        let guard = self.entries.try_read().or(Err(CacheError))?;

        Ok(guard.iter().enumerate().find_map(|(index, (key, value))| {
            if path == &**key {
                Some((index, value.to_vec()))
            } else {
                None
            }
        }))
    }

    pub fn write(&self, path: &str, matches: &[Option<Match>]) {
        if let Ok(mut guard) = self.entries.try_write() {
            if self.capacity == guard.len() {
                guard.pop_back();
            }

            guard.push_front((path.into(), matches.to_vec()));
        }
    }
}
