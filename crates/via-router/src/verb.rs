use http::method::Method;
use std::{iter::FusedIterator, ops::BitOr, slice::Iter};

#[derive(Debug)]
pub struct Get<'a, T> {
    verb: Verb,
    iter: Iter<'a, (Verb, T)>,
}

#[derive(Clone, Debug)]
pub struct Map<T> {
    entries: Vec<(Verb, T)>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Verb(u16);

impl<'a, T> FusedIterator for Get<'a, T> {}

impl<'a, T> Iterator for Get<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let (other, value) = self.iter.next()?;

        if other.intersects(self.verb) {
            Some(value)
        } else {
            self.next()
        }
    }
}

impl<T> Map<T> {
    pub fn new() -> Map<T> {
        Default::default()
    }

    pub fn get(&self, verb: impl Into<Verb>) -> Get<T> {
        let verb = verb.into();
        let iter = if verb.0 == 0 {
            (&[]).iter()
        } else {
            self.entries.iter()
        };

        Get { iter, verb }
    }

    pub fn insert(&mut self, verb: impl Into<Verb>, value: T) -> &mut T {
        let key = verb.into();
        let index = match self.entries.iter().position(|(k, _)| *k == key) {
            Some(index) => {
                self.entries[index] = (key, value);
                index
            }
            None => {
                self.entries.push((key, value));
                self.entries.len() - 1
            }
        };

        &mut self.entries[index].1
    }
}

impl<T> Default for Map<T> {
    fn default() -> Map<T> {
        Map {
            entries: Default::default(),
        }
    }
}

impl Verb {
    pub const CONNECT: Verb = Verb(0b0_0000_0001);
    pub const DELETE: Verb = Verb(0b0_0000_0010);
    pub const GET: Verb = Verb(0b0_0000_0100);
    pub const HEAD: Verb = Verb(0b0_0000_1000);
    pub const OPTIONS: Verb = Verb(0b0_0001_0000);
    pub const PATCH: Verb = Verb(0b0_0010_0000);
    pub const POST: Verb = Verb(0b0_0100_0000);
    pub const PUT: Verb = Verb(0b0_1000_0000);
    pub const TRACE: Verb = Verb(0b1_0000_0000);

    pub const fn all() -> Verb {
        Verb(0b1_1111_1111)
    }

    pub const fn none() -> Verb {
        Verb(0)
    }

    pub fn intersects(self, other: Verb) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for Verb {
    type Output = Verb;

    fn bitor(self, other: Verb) -> Self::Output {
        Verb(self.0 | other.0)
    }
}

impl From<Method> for Verb {
    fn from(method: Method) -> Verb {
        Verb::from(&method)
    }
}

impl<'a> From<&'a Method> for Verb {
    fn from(method: &'a Method) -> Verb {
        match *method {
            Method::CONNECT => Verb::CONNECT,
            Method::DELETE => Verb::DELETE,
            Method::GET => Verb::GET,
            Method::HEAD => Verb::HEAD,
            Method::OPTIONS => Verb::OPTIONS,
            Method::PATCH => Verb::PATCH,
            Method::POST => Verb::POST,
            Method::PUT => Verb::PUT,
            Method::TRACE => Verb::TRACE,
            _ => Verb::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Verb;

    type Map = super::Map<&'static str>;

    #[test]
    fn insert_order() {
        let mut verbs = Map::new();

        verbs.insert(Verb::GET, "GET");
        verbs.insert(Verb::POST, "POST");
        verbs.insert(Verb::all(), "ALL");

        assert_eq!(verbs.get(Verb::GET).next(), Some(&"GET"));
        assert_eq!(verbs.get(Verb::POST).next(), Some(&"POST"));
        assert_eq!(verbs.get(Verb::DELETE).next(), Some(&"ALL"));
    }
}
