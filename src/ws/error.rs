use std::fmt::{self, Display, Formatter};
use std::ops::ControlFlow::{Break, Continue};
use tokio_websockets::Error as WebSocketError;

use crate::error::Error;

type ControlFlow<T> = std::ops::ControlFlow<T, T>;
pub type Result<T = ()> = std::result::Result<T, ControlFlow<Error>>;

pub trait Retry {
    type Output;
    fn or_break(self) -> Result<Self::Output>;
    fn or_continue(self) -> Result<Self::Output>;
}

#[derive(Debug)]
pub enum ErrorKind {
    Runtime(Error),
    Socket(WebSocketError),
}

pub fn try_rescue_ws(error: WebSocketError) -> ControlFlow<Option<ErrorKind>> {
    let into_control_flow = match &error {
        WebSocketError::PayloadTooLong { .. } | WebSocketError::Protocol(_) => Continue,
        WebSocketError::Io(source) => match source.kind() {
            std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::WriteZero => Continue,
            _ => Break,
        },
        _ => Break,
    };

    into_control_flow(Some(error.into()))
}

impl ErrorKind {
    pub const CLOSED: Self = Self::Socket(WebSocketError::AlreadyClosed);
}

impl std::error::Error for ErrorKind {}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Runtime(error) => Display::fmt(error, f),
            Self::Socket(error) => Display::fmt(error, f),
        }
    }
}

impl From<Error> for ErrorKind {
    fn from(error: Error) -> Self {
        Self::Runtime(error)
    }
}

impl From<WebSocketError> for ErrorKind {
    fn from(error: WebSocketError) -> Self {
        Self::Socket(error)
    }
}

impl<T> Retry for crate::Result<T> {
    type Output = T;

    #[inline]
    fn or_break(self) -> Result<Self::Output> {
        self.map_err(Break)
    }

    #[inline]
    fn or_continue(self) -> Result<Self::Output> {
        self.map_err(Continue)
    }
}
