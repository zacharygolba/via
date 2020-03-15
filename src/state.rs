use crate::{http::Extensions, Result};
use std::sync::Arc;

/// A marker trait used to describe types that can be injected into the global
/// state of an application.
pub trait Value: Send + Sync + 'static {}

#[derive(Debug, Default)]
pub struct State {
    entries: Arc<Extensions>,
}

impl State {
    pub fn get<T: Value>(&self) -> Result<&T> {
        if let Some(value) = self.entries.get() {
            Ok(value)
        } else {
            todo!()
        }
    }

    pub fn insert(&mut self, value: impl Value) {
        let entries = match Arc::get_mut(&mut self.entries) {
            Some(value) => value,
            None => todo!(),
        };

        entries.insert(value);
    }
}

impl Clone for State {
    #[inline]
    fn clone(&self) -> Self {
        State {
            entries: Arc::clone(&self.entries),
        }
    }
}

impl<T: Send + Sync + 'static> Value for T {}
