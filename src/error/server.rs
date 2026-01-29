use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::io;

use super::BoxError;

#[derive(Debug)]
struct HandshakeTimeoutError;

#[derive(Debug)]
pub(crate) enum ServerError {
    Http(hyper::Error),
    Other(BoxError),
}

impl Error for HandshakeTimeoutError {}

impl Display for HandshakeTimeoutError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "tls negotiation did not finish within the configured timeout",
        )
    }
}

impl ServerError {
    #[allow(dead_code)]
    pub fn handshake_timeout() -> Self {
        Self::Other(Box::new(HandshakeTimeoutError))
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Http(error) => Display::fmt(error, f),
            Self::Other(error) => Display::fmt(error, f),
        }
    }
}

impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Http(error) => error.source(),
            Self::Other(error) => error.source(),
        }
    }
}

impl From<hyper::Error> for ServerError {
    fn from(error: hyper::Error) -> Self {
        Self::Http(error)
    }
}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        Self::Other(Box::new(error))
    }
}

#[cfg(feature = "native-tls")]
impl From<native_tls::Error> for ServerError {
    fn from(error: native_tls::Error) -> Self {
        Self::Other(Box::new(error))
    }
}
