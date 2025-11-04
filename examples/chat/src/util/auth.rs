use http::Extensions;
use via::request::RequestHead;
use via::{Middleware, raise, ws};

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
        let jar = request.cookies().private(request.state().secret());

        if let Some(cookie) = jar.get(&self.cookie)
            && let Ok(user) = serde_json::from_str(cookie.value())
        {
            request.extensions_mut().insert(Verify(user));
        }

        next.call(request)
    }
}

impl Authenticate for Request {
    fn current_user(&self) -> via::Result<&User> {
        try_from_extensions(self.extensions())
    }
}

impl Authenticate for ws::Request<Chat> {
    fn current_user(&self) -> via::Result<&User> {
        try_from_extensions(self.extensions())
    }
}

impl Authenticate for RequestHead<Chat> {
    fn current_user(&self) -> via::Result<&User> {
        try_from_extensions(self.extensions())
    }
}
