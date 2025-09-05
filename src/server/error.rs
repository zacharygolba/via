use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    Hyper(hyper::Error),
}

impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => error.source(),
            Self::Hyper(error) => error.source(),
        }
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, f),
            Self::Hyper(error) => Display::fmt(error, f),
        }
    }
}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<hyper::Error> for ServerError {
    fn from(error: hyper::Error) -> Self {
        Self::Hyper(error)
    }
}
