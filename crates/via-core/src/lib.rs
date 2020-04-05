#[macro_use]
pub mod error;
pub mod middleware;
pub mod response;
pub mod routing;

#[doc(inline)]
pub use self::{
    error::{Error, Result},
    middleware::{Context, Middleware, Next},
    response::{Respond, Response},
};

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
