use diesel::prelude::*;
use serde::Serialize;
use via::{Payload, Response};

use crate::models::message::{self, MessageWithAuthor};
use crate::models::subscription::{AuthClaims, NewSubscription, UserSubscription};
use crate::models::{Message, Subscription, Thread, User};
use crate::util::auth::access_denied;
use crate::util::{DebugQueryDsl, LimitAndOffset, Session};
use crate::{Next, Request};

#[derive(Serialize)]
struct ShowThread {
    #[serde(flatten)]
    thread: Thread,
    messages: Vec<MessageWithAuthor>,
    subscriptions: Vec<UserSubscription>,
}

pub async fn index(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.envelope().current_user()?.id;

    // Get pagination params from the URI query.
    let LimitAndOffset(limit, offset) = request.envelope().query()?;

    // Acquire a database connection and execute the query.
    let threads: Vec<Thread> = {
        let mut connection = request.state().pool().get().await?;

        Thread::by_participant(&current_user_id)
            .limit(limit)
            .offset(offset)
            .debug_load(&mut connection)
            .await?
    };

    Response::build().json(&threads)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.envelope().current_user()?.id;

    // Deserialize the request body into thread params.
    let (body, state) = request.into_future();
    let new_thread = body.await?.json()?;

    // Acquire a database connection.
    let mut connection = state.pool().get_owned().await?;

    // Insert the thread into the threads table.
    let thread = Thread::create(new_thread)
        .returning(Thread::as_returning())
        .debug_result(&mut connection)
        .await?;

    // Subscribe the current user to the thread as an an admin.
    let new_subscription = NewSubscription {
        claims: AuthClaims::all(),
        user_id: current_user_id,
        thread_id: Some(*thread.id()),
    };

    // Insert the subscription.
    Subscription::create(new_subscription)
        .returning(Subscription::as_returning())
        .debug_result(&mut connection)
        .await?;

    Response::build().status(201).json(&thread)
}

/// Confirm that the current user is subscribed to the thread.
///
pub async fn authorization(mut request: Request, next: Next) -> via::Result {
    let current_user = request.envelope().current_user()?;
    let thread_id = request.envelope().param("thread-id").parse()?;

    // Acquire a database connection and execute the query.
    let Some(subscription) = ({
        let mut connection = request.state().pool().get().await?;
        let result = Subscription::select()
            .filter(Subscription::by_join(&current_user.id, &thread_id))
            .filter(Subscription::user_can_participate())
            .debug_first(&mut connection)
            .await;

        result.optional()?
    }) else {
        return access_denied();
    };

    // Insert the subscription in request extensions so it can be used later.
    request.envelope_mut().extensions_mut().insert(subscription);

    // Call the next middleware.
    next.call(request).await
}

/// Get a reference to the current user's thread subscription from request
/// extensions.
///
pub fn subscription_for(request: &Request) -> via::Result<&Subscription> {
    match request.envelope().extensions().get() {
        Some(subscription) => Ok(subscription),
        None => access_denied(),
    }
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the thread.
    let subscription = subscription_for(&request)?;

    // Acquire a database connection.
    let mut connection = request.state().pool().get_owned().await?;

    // Load the thread with :thread-id through the current user's subscription.
    let thread = Thread::by_subscription(subscription)
        .debug_first(&mut connection)
        .await?;

    // Load the first page of messages in thread.
    let messages = Message::belonging_to(&thread)
        .inner_join(User::TABLE)
        .select((Message::as_select(), User::as_select()))
        .order(message::created_at_desc())
        .limit(25)
        .debug_load(&mut connection)
        .await?;

    let subscriptions = Subscription::belonging_to(&thread)
        .inner_join(User::TABLE)
        .select((Subscription::as_select(), User::as_select()))
        .limit(10)
        .debug_load(&mut connection)
        .await?;

    Response::build().json(&ShowThread {
        thread,
        messages,
        subscriptions,
    })
}

pub async fn update(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the thread.
    let subscription = subscription_for(&request)?;
    let thread_id = *subscription.thread_id();

    // Confirm that the current user can update the thread.
    if !subscription.claims().contains(AuthClaims::MODERATE) {
        return access_denied();
    }

    // Deserialize the request body into a thread change set.
    let (body, state) = request.into_future();
    let change_set = body.await?.json()?;

    // Acquire a database connection and execute the update.
    let thread = Thread::update(&thread_id, change_set)
        .returning(Thread::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    Response::build().json(&thread)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the thread.
    let subscription = subscription_for(&request)?;

    // Confirm that the current user can delete the thread.
    if !subscription.claims().contains(AuthClaims::MODERATE) {
        return access_denied();
    }

    // Acquire a database connection and execute the delete.
    Thread::delete(subscription.thread_id())
        .debug_execute(&mut request.state().pool().get().await?)
        .await?;

    Response::build().status(204).finish()
}
