use diesel::prelude::*;

use crate::models::subscription::{AuthClaims, Subscription, ThreadSubscription};
use crate::util::{DebugQueryDsl, Session, auth};
use crate::{Next, Request};

#[derive(Clone)]
struct Verify(ThreadSubscription);

/// Granular permission guards for subscribers.
pub trait Ability {
    type Output;

    /// Returns a result containing the associated Output if the subscriber has
    /// the provided claims.
    fn can(&self, claims: AuthClaims) -> via::Result<&Self::Output>;
}

/// Has a subscription to a thread.
pub trait Subscriber {
    /// Return's the current user's subscription to the thread.
    fn subscription(&self) -> via::Result<&ThreadSubscription>;
}

/// Confirm that the current user is subscribed to the thread.
pub async fn authorization(mut request: Request, next: Next) -> via::Result {
    let current_user_id = request.current_user()?.id;
    let thread_id = request.envelope().param("thread-id").parse()?;

    // Acquire a database connection and execute the query.
    let Some(subscription) = ThreadSubscription::select()
        .filter(Subscription::by_user(&current_user_id))
        .filter(Subscription::by_thread(&thread_id))
        .filter(Subscription::can_participate())
        .debug_first(&mut request.state().pool().get().await?)
        .await
        .optional()?
    else {
        return auth::access_denied();
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
    type Output = Self;

    fn can(&self, claims: AuthClaims) -> via::Result<&Self::Output> {
        if self.claims().contains(claims) {
            Ok(self)
        } else {
            auth::access_denied()
        }
    }
}

impl Ability for Request {
    type Output = ThreadSubscription;

    /// Returns the current user's subscription to the thread if they have the
    /// provided claims.
    ///
    fn can(&self, claims: AuthClaims) -> via::Result<&Self::Output> {
        self.subscription()?.can(claims)
    }
}

impl Subscriber for Request {
    fn subscription(&self) -> via::Result<&ThreadSubscription> {
        match self.envelope().extensions().get() {
            Some(Verify(subscription)) => Ok(subscription),
            None => auth::access_denied(),
        }
    }
}
