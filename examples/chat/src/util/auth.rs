use cookie::{Cookie, Key, SameSite};
use time::{Duration, OffsetDateTime};
use via::request::Envelope;
use via::{Middleware, Response, raise, ws};

use crate::chat::Chat;
use crate::models::user::User;
use crate::{Next, Request};

pub const SESSION: &str = "via-chat-session";

pub trait Authenticate {
    fn set_current_user(&mut self, secret: &Key, user: Option<&User>) -> via::Result<()>;
}

pub trait Session {
    fn current_user(&self) -> via::Result<&User>;

    fn is_authenticated(&self) -> via::Result<()> {
        self.current_user().and(Ok(()))
    }
}

pub struct RestoreSession;

/// Prevents the user type from being accessed in extensions outside this
/// module.
///
#[derive(Clone)]
struct Verify(User);

pub fn access_denied<T>() -> via::Result<T> {
    raise!(403, message = "Access denied.");
}

pub fn unauthorized<T>() -> via::Result<T> {
    raise!(401, message = "Authentication is required.");
}

impl RestoreSession {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware<Chat> for RestoreSession {
    fn call(&self, mut request: Request, next: Next) -> via::BoxFuture {
        if let Some(cookie) = request
            .envelope()
            .cookies()
            .private(request.state().secret())
            .get(SESSION)
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

impl Authenticate for Response {
    fn set_current_user(&mut self, secret: &Key, user: Option<&User>) -> via::Result<()> {
        // Build an empty session cookie.
        let mut session = Cookie::build(SESSION)
            .http_only(true)
            .same_site(SameSite::Strict)
            .expires(OffsetDateTime::now_utc() + Duration::weeks(2))
            .secure(true)
            .path("/")
            .build();

        if let Some(value) = user {
            // Set the value of the cookie to the user as JSON.
            session.set_value(serde_json::to_string(value)?);
        } else {
            // Indicates to the client that the cookie should be removed.
            session.make_removal();
        };

        // Add the session cookie.
        self.cookies_mut().private_mut(secret).add(session);

        Ok(())
    }
}

fn get_current_user(envelope: &Envelope) -> via::Result<&User> {
    match envelope.extensions().get() {
        Some(Verify(user)) => Ok(user),
        None => unauthorized(),
    }
}

impl Session for Request {
    fn current_user(&self) -> via::Result<&User> {
        get_current_user(self.envelope())
    }
}

impl Session for ws::Request<Chat> {
    fn current_user(&self) -> via::Result<&User> {
        get_current_user(self.envelope())
    }
}
