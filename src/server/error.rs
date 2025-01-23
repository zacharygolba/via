use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct ServiceError;

impl Error for ServiceError {}

impl Display for ServiceError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "internal server error")
    }
}
