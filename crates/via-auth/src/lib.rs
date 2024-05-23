pub mod basic;
pub mod login;
pub mod prelude {
    pub use super::{ContextExt, Session};
}

use core::{BoxFuture, Context, Error, Middleware, Next, Result};
use std::future::Future;

use self::basic::BasicStrategy;

type AuthResult<T = CurrentUser> = Result<Option<T>, Error>;
type CurrentUser = ();

pub trait ContextExt {
    fn session(&self) -> Result<&Session>;
}

pub trait Strategy: Send + Sync + 'static {
    type Future: Future<Output = AuthResult<Self::User>> + Send + 'static;
    type User: Send + Sync + 'static;

    fn authenticate(&self, context: &Context) -> Self::Future;
}

pub struct Authenticate<T: Strategy> {
    strategy: T,
}

#[derive(Clone, Default)]
pub struct Session {
    user: Option<CurrentUser>,
}

pub fn basic<F, T, U>(login: F) -> Authenticate<impl Strategy>
where
    F: Fn(String, String) -> T + Send + Sync + 'static,
    T: Future<Output = AuthResult<U>> + Send + 'static,
    U: Send + Sync + 'static,
{
    Authenticate {
        strategy: BasicStrategy { login },
    }
}

impl ContextExt for Context {
    fn session(&self) -> Result<&Session> {
        match self.get::<Session>() {
            Err(_) => error::bail!("TODO(@zacharygolba): add error message"),
            result => result,
        }
    }
}

impl<T: Strategy> Middleware for Authenticate<T> {
    fn call(&self, mut context: Context, next: Next) -> BoxFuture<Result> {
        let future = self.strategy.authenticate(&context);

        context.insert(Session::empty());
        unimplemented!()
        // Box::pin(async move {
        //     if let Some(user) = future.await? {
        //         context.insert(Session::new(user));
        //         next.call(context).await
        //     } else {
        //         error::status!(401, "Unauthorized")
        //     }
        // })
    }
}

impl Session {
    fn empty() -> Self {
        Session { user: None }
    }

    fn new(user: impl Send + Sync + 'static) -> Self {
        Session { user: Some(()) }
    }
}
