use core::{BoxFuture, Error};
use std::future::Future;

pub trait Login: Send + Sync + 'static {
    type User: Send + Sync + 'static;
    type Error: Into<Error> + Send + 'static;
    type Future: Future<Output = Result<Self::User, Self::Error>>;

    fn login(&self, credentials: Credentials) -> Self::Future;
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}
