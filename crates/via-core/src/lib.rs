pub mod middleware;
pub mod response;
pub mod router;

pub use error::Error;

#[doc(inline)]
pub use self::{
    middleware::{Context, Middleware, Next},
    response::Respond,
};

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
pub type Result<T = response::Response, E = Error> = std::result::Result<T, E>;
