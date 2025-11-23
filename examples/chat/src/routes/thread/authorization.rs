use diesel::prelude::*;

use crate::models::subscription::*;
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, Id, Session};
use crate::{Next, Request};

#[derive(Clone)]
struct Verify(ThreadSubscription);

/// Granular permission guards for subscribers.
pub trait Ability {
    type Output;
    type Error;

    /// Returns a result containing the associated Output if the subscriber has
    /// the provided claims.
    fn can(&self, claims: AuthClaims) -> Result<&Self::Output, Self::Error>;
}

/// Has a subscription to a thread.
pub trait Subscriber {
    /// Return's the current user's subscription to the thread.
    fn subscription(&self) -> via::Result<&ThreadSubscription>;
}

/// Confirm that the current user is subscribed to the thread.
pub async fn authorization(mut request: Request, next: Next) -> via::Result {
    let user_id = request.user()?;
    let thread_id = request.envelope().param("thread-id").parse()?;

    // Acquire a database connection and execute the query.
    let Some(subscription) = Subscription::threads()
        .select(ThreadSubscription::as_select())
        .filter(by_user(user_id).and(by_thread(&thread_id)))
        .filter(claims_can_participate())
        .debug_first(&mut request.app().pool().get().await?)
        .await
        .optional()?
    else {
        return forbidden();
    };

    // Insert the subscription in request extensions so it can be used later.
    request
        .envelope_mut()
        .extensions_mut()
        .insert(Verify(subscription));

    // Call the next middleware.
    next.call(request).await
}

impl Ability for ThreadSubscription {
    type Output = Id;
    type Error = Id;

    fn can(&self, claims: AuthClaims) -> Result<&Self::Output, Self::Error> {
        if self.claims().contains(claims) {
            Ok(self.thread().id())
        } else {
            Err(*self.user_id())
        }
    }
}

impl Ability for Request {
    type Output = Id;
    type Error = via::Error;

    /// Returns the current user's subscription to the thread if they have the
    /// provided claims.
    ///
    fn can(&self, claims: AuthClaims) -> Result<&Self::Output, Self::Error> {
        self.subscription()?.can(claims).or_else(|_| forbidden())
    }
}

impl Subscriber for Request {
    fn subscription(&self) -> via::Result<&ThreadSubscription> {
        match self.envelope().extensions().get() {
            Some(Verify(subscription)) => Ok(subscription),
            None => forbidden(),
        }
    }
}
