use std::collections::VecDeque;
use std::sync::RwLock;

use crate::router::Match;

pub struct Cache {
    capacity: usize,
    entries: RwLock<VecDeque<(Box<str>, Vec<Option<Match>>)>>,
}

#[inline]
fn fetch(
    store: &VecDeque<(Box<str>, Vec<Option<Match>>)>,
    key: &str,
) -> Option<(usize, Vec<Option<Match>>)> {
    let (index, matches) = store.iter().enumerate().find_map(|(i, (k, matches))| {
        if *key == **k {
            Some((i, matches))
        } else {
            None
        }
    })?;

    Some((index, matches.to_vec()))
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: RwLock::new(VecDeque::with_capacity(capacity)),
        }
    }

    pub fn read(&self, path: &str) -> Option<Option<Vec<Option<Match>>>> {
        let lock = &self.entries;
        let cap = self.capacity;

        #[allow(clippy::never_loop)]
        let (index, matches) = loop {
            return match lock.try_read() {
                Ok(guard) => match fetch(&guard, path) {
                    Some(existing) => break existing,
                    None => Some(None),
                },
                Err(_) => None,
            };
        };

        if cap
            .checked_div(2)
            .map_or(false, |threshold| index > threshold)
        {
            if let Ok(mut guard) = lock.try_write() {
                guard.swap_remove_front(index);
            }
        }

        Some(Some(matches))
    }

    pub fn write(&self, path: Box<str>, matches: Vec<Option<Match>>) {
        if let Ok(mut guard) = self.entries.try_write() {
            if self.capacity == guard.len() {
                guard.pop_back();
            }

            guard.push_front((path, matches));
        }
    }
}
