use std::collections::VecDeque;
use std::error::Error;
use std::sync::RwLock;

use crate::router::Match;

pub struct Cache {
    capacity: usize,
    #[allow(clippy::type_complexity)]
    entries: RwLock<VecDeque<(Box<str>, Vec<Match>)>>,
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

    #[allow(clippy::type_complexity)]
    pub fn read(
        &self,
        path: &str,
    ) -> Result<Option<(usize, Vec<Match>)>, Box<dyn Error + Send + Sync>> {
        let guard = self.entries.try_read().or(Err("lock in use"))?;

        Ok(guard.iter().enumerate().find_map(|(index, (key, value))| {
            if path == &**key {
                Some((index, value.to_vec()))
            } else {
                None
            }
        }))
    }

    pub fn write(&self, path: &str, matches: &[Match]) {
        if let Ok(mut guard) = self.entries.try_write() {
            if self.capacity == guard.len() {
                guard.pop_back();
            }

            guard.push_front((path.into(), matches.to_vec()));
        }
    }
}
