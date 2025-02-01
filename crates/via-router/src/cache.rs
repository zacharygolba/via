use std::collections::VecDeque;
use std::sync::RwLock;

use crate::search::Match;

pub struct Cache {
    capacity: usize,
    entries: RwLock<VecDeque<(Box<str>, Vec<Match>)>>,
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: RwLock::new(VecDeque::with_capacity(capacity)),
        }
    }

    pub fn try_read(&self, path: &str) -> Option<Option<Vec<Match>>> {
        let lock = &self.entries;
        let cached = {
            let guard = match lock.try_read() {
                Ok(guard) => guard,
                Err(_) => return None,
            };

            guard.iter().enumerate().find_map(|(index, (key, cached))| {
                if **key == *path {
                    Some((index, cached.to_vec()))
                } else {
                    None
                }
            })
        };

        match cached {
            None => Some(None),
            Some((index, matches)) => {
                if index > self.capacity.div_ceil(2) {
                    if let Ok(mut guard) = lock.try_write() {
                        guard.swap_remove_front(index);
                    }
                }

                Some(Some(matches))
            }
        }
    }

    pub fn try_write(&self, path: &str, matches: &Vec<Match>) {
        if let Ok(mut guard) = self.entries.try_write() {
            if guard.len() == self.capacity {
                guard.pop_back();
            }

            guard.push_front((path.into(), matches.to_vec()));
        }
    }
}
