use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
};

pub struct Message {
    pub value: String,
}

#[macro_export]
macro_rules! bail {
    ($($tokens:tt)+) => {
        Err($crate::Message {
            value: format!($($tokens)+)
        })?
    };
}

#[macro_export]
macro_rules! status {
    ($code:expr, $($tokens:tt)+) => {{
        let message = $crate::Message {
            value: format!($($tokens)+)
        };

        return Err($crate::Error::from(message).status($code))
    }};
}

impl Debug for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl StdError for Message {}
