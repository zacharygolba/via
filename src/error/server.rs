use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::io;
use tokio::task::JoinError;

#[derive(Debug)]
pub(crate) enum ServerError {
    Io(io::Error),
    Join(JoinError),
    Http(hyper::Error),

    #[cfg(feature = "native-tls")]
    Tls(native_tls::Error),
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, f),
            Self::Join(error) => Display::fmt(error, f),
            Self::Http(error) => Display::fmt(error, f),

            #[cfg(feature = "native-tls")]
            Self::Tls(error) => Display::fmt(error, f),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => error.source(),
            Self::Join(error) => error.source(),
            Self::Http(error) => error.source(),

            #[cfg(feature = "native-tls")]
            Self::Tls(error) => error.source(),
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
        Self::Http(error)
    }
}

#[cfg(feature = "native-tls")]
impl From<native_tls::Error> for ServerError {
    fn from(error: native_tls::Error) -> Self {
        Self::Tls(error)
    }
}
