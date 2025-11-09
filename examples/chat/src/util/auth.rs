use http::Extensions;
use via::request::Head;
use via::{Middleware, raise};

use crate::chat::Chat;
use crate::models::user::User;
use crate::{Next, Request};

pub trait Authenticate {
    fn current_user(&self) -> via::Result<&User>;
}

pub struct Auth {
    cookie: String,
}

#[derive(Clone)]
struct Verify(User);

#[inline]
fn try_from_extensions(extensions: &Extensions) -> via::Result<&User> {
    let Some(Verify(user)) = extensions.get() else {
        raise!(401);
    };

    Ok(user)
}

impl Auth {
    pub fn new(cookie: impl Into<String>) -> Self {
        Self {
            cookie: cookie.into(),
        }
    }
}

impl Middleware<Chat> for Auth {
    fn call(&self, mut request: Request, next: Next) -> via::BoxFuture {
        let session = {
            let envelope = request.head();

            envelope
                .cookies()
                .private(request.state().secret())
                .get(&self.cookie)
                .map(|cookie| serde_json::from_str(cookie.value()))
        };

        if let Some(result) = session {
            let extension = match result {
                Err(error) => return Box::pin(async { raise!(400, error) }),
                Ok(user) => Verify(user),
            };

            request.head_mut().extensions_mut().insert(extension);
        }

        next.call(request)
    }
}

impl Authenticate for Head {
    fn current_user(&self) -> via::Result<&User> {
        try_from_extensions(self.extensions())
    }
}
