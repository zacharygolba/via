pub mod article;

use std::collections::HashMap;
use tokio::sync::RwLock;

pub use self::article::Article;
pub use uuid::Uuid;

pub type Store<T> = RwLock<Data<T>>;

#[derive(Debug)]
pub struct Data<T> {
    entries: HashMap<Uuid, T>,
}

impl<T> Data<T> {
    pub fn all(&self) -> Vec<&T> {
        self.entries.values().collect()
    }

    pub fn find(&self, id: &Uuid) -> Option<&T> {
        self.entries.get(id)
    }

    pub fn insert(&mut self, f: impl FnOnce(Uuid) -> T) -> &T {
        let key = Uuid::new_v4();
        let value = f(key.clone());

        self.entries.insert(key.clone(), value);
        &self.entries[&key]
    }

    pub fn remove(&mut self, id: &Uuid) -> Option<T> {
        self.entries.remove(id)
    }

    pub fn update(&mut self, id: &Uuid, f: impl FnOnce(&mut T)) -> Option<&T> {
        f(self.entries.get_mut(id)?);
        self.entries.get(id)
    }
}

impl<T> Default for Data<T> {
    fn default() -> Data<T> {
        Data {
            entries: HashMap::new(),
        }
    }
}
