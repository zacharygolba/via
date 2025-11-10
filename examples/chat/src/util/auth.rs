use http::Extensions;
use via::request::Envelope;
use via::{Middleware, raise};

use crate::chat::Chat;
use crate::models::user::User;
use crate::{Next, Request};

pub trait Session {
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
        if let Some(cookie) = request
            .envelope()
            .cookies()
            .private(request.state().secret())
            .get(&self.cookie)
        {
            let current_user = match serde_json::from_str(cookie.value()) {
                Err(error) => return Box::pin(async { raise!(400, error) }),
                Ok(user) => user,
            };

            request
                .envelope_mut()
                .extensions_mut()
                .insert(Verify(current_user));
        }

        next.call(request)
    }
}

impl Session for Envelope {
    fn current_user(&self) -> via::Result<&User> {
        try_from_extensions(self.extensions())
    }
}
