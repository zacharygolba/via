use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug)]
pub struct Error(ErrorKind);

#[derive(Clone, Copy, Debug)]
enum ErrorKind {
    Path,
    Router,
}

impl Error {
    pub fn path() -> Self {
        Self(ErrorKind::Path)
    }

    pub fn router() -> Self {
        Self(ErrorKind::Router)
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.0 {
            ErrorKind::Path => {
                write!(f, "path segment range is out of bounds")
            }
            ErrorKind::Router => {
                write!(f, "router node index out of bounds")
            }
        }
    }
}
