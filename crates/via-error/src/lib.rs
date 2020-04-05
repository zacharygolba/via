mod macros;
mod respond;

use self::respond::*;
use std::{
    error::Error as StdError,
    fmt::{self, Display, Formatter},
};

#[doc(hidden)]
pub use self::macros::Message;

pub type Source = (dyn StdError + 'static);

pub trait ResultExt<T> {
    fn json(self) -> Result<T, Error>;
    fn status(self, code: u16) -> Result<T, Error>;
}

#[derive(Debug)]
pub struct Error {
    respond: Respond,
    source: Box<dyn StdError + Send>,
}

#[derive(Clone, Copy, Debug)]
pub struct Iter<'a> {
    source: Option<&'a Source>,
}

impl Error {
    pub fn chain(&self) -> impl Iterator<Item = &Source> {
        Iter {
            source: Some(&*self.source),
        }
    }

    pub fn json(mut self) -> Self {
        self.respond.format = Some(Format::Json);
        self
    }

    pub fn source(&self) -> &Source {
        &*self.source
    }

    pub fn status(mut self, code: u16) -> Self {
        self.respond.status = code;
        self
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.source, f)
    }
}

impl<T> From<T> for Error
where
    T: StdError + Send + 'static,
{
    fn from(value: T) -> Self {
        Error {
            respond: Default::default(),
            source: Box::new(value),
        }
    }
}

impl<'a> IntoIterator for &'a Error {
    type IntoIter = Iter<'a>;
    type Item = &'a Source;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            source: Some(&*self.source),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Source;

    fn next(&mut self) -> Option<Self::Item> {
        self.source.map(|error| {
            self.source = error.source();
            error
        })
    }
}

impl From<Error> for Box<dyn StdError + Send> {
    fn from(error: Error) -> Self {
        error.source
    }
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    Error: From<E>,
{
    fn json(self) -> Result<T, Error> {
        self.map_err(|e| Error::from(e).json())
    }

    fn status(self, code: u16) -> Result<T, Error> {
        self.map_err(|e| Error::from(e).status(code))
    }
}
