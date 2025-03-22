use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl std::error::Error for Error {}

impl Error {
    pub(crate) fn new() -> Self {
        Self {
            message: "an error occurred when routing the request".to_owned(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}
