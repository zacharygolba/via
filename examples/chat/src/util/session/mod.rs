mod identity;

use cookie::{Cookie, Key, SameSite};
use diesel::dsl::count;
use diesel::prelude::*;
use http::Extensions;
use time::{Duration, OffsetDateTime};
use via::{Response, ws};

use self::identity::Identity;
use super::error::unauthorized;
use crate::chat::Chat;
use crate::models::user::*;
use crate::schema::users;
use crate::util::{DebugQueryDsl, Id};
use crate::{Next, Request};

/// Prevents the user type from being accessed in extensions outside this
/// module.
///
#[derive(Clone)]
struct Verify(Id);

pub const COOKIE: &str = "via-chat-session";

pub trait Authenticate {
    fn set_user(&mut self, secret: &Key, user: Option<Id>) -> via::Result<()>;
}

pub trait Session {
    fn user(&self) -> via::Result<&Id>;

    /// Returns an error if there is no user associated with the request.
    ///
    fn authenticate(&self) -> via::Result<()> {
        self.user().and(Ok(()))
    }
}

pub async fn restore(mut request: Request, next: Next) -> via::Result {
    let mut refresh = None;
    let state = request.state().clone();

    if let Some(identity) = request
        .envelope()
        .cookies()
        .signed(state.secret())
        .get(COOKIE)
        .map(|cookie| cookie.value().parse::<Identity>())
        .transpose()?
    {
        if identity.is_expired() {
            if let ..1i64 = User::table()
                .select(count(users::id))
                .filter(by_id(identity.id()))
                .debug_result(&mut state.pool().get().await?)
                .await?
            {
                return unauthorized();
            }

            refresh = Some(*identity.id());
        }

        request
            .envelope_mut()
            .extensions_mut()
            .insert(Verify(*identity.id()));
    }

    let mut response = next.call(request).await?;

    if let Some(id) = refresh
        && response.status().is_success()
    {
        response.set_user(state.secret(), Some(id))?;
    }

    Ok(response)
}

fn identify(extensions: &Extensions) -> via::Result<&Id> {
    if let Some(Verify(id)) = extensions.get() {
        Ok(id)
    } else {
        unauthorized()
    }
}

impl Session for Request {
    fn user(&self) -> via::Result<&Id> {
        identify(self.envelope().extensions())
    }
}

impl Session for ws::Request<Chat> {
    fn user(&self) -> via::Result<&Id> {
        identify(self.envelope().extensions())
    }
}

impl Authenticate for Response {
    fn set_user(&mut self, secret: &Key, user: Option<Id>) -> via::Result<()> {
        // Build an empty session cookie.
        let mut session = Cookie::build(COOKIE)
            .http_only(true)
            .same_site(SameSite::Strict)
            .expires(OffsetDateTime::now_utc() + Duration::weeks(2))
            .secure(true)
            .path("/")
            .build();

        if let Some(id) = user {
            // Set the value of the cookie to the user.
            session.set_value(Identity::new(id).encode());
        } else {
            // Indicates to the client that the cookie should be removed.
            session.make_removal();
        };

        // Add the session cookie.
        self.cookies_mut().signed_mut(secret).add(session);

        Ok(())
    }
}
