pub mod messages;
pub mod reactions;
pub mod subscriptions;

pub use authorization::authorization;

mod authorization;

use diesel::prelude::*;
use via::{Payload, Response};

use self::authorization::{Ability, Subscriber};
use crate::models::subscription::{AuthClaims, NewSubscription, UserSubscription};
use crate::models::thread::ThreadIncludes;
use crate::models::{Message, Subscription, Thread};
use crate::util::{DebugQueryDsl, LimitAndOffset, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.current_user()?.id;

    // Get pagination params from the URI query.
    let LimitAndOffset(limit, offset) = request.envelope().query()?;

    // Acquire a database connection and execute the query.
    let threads: Vec<Thread> = {
        let mut connection = request.state().pool().get().await?;

        Thread::subscriptions()
            .filter(Subscription::by_user(&current_user_id))
            .order(Thread::created_at_desc())
            .limit(limit)
            .offset(offset)
            .debug_load(&mut connection)
            .await?
    };

    Response::build().json(&threads)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let current_user = request.current_user().cloned()?;

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

    // Associate the current user to the thread as an admin.
    let rows = Subscription::create(NewSubscription::admin(&current_user, &thread))
        .debug_execute(&mut connection)
        .await?;

    // In debug builds assert that the number of affected rows is not zero.
    debug_assert!(rows > 0, "unable to create subscription for thread admin");

    Response::build().status(201).json(&thread)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // Clone the thread that we eagerly loaded during authorization.
    let thread = request.subscription()?.thread.clone();

    // Acquire a database connection.
    let mut connection = request.state().pool().get_owned().await?;

    // Load the 25 most recent chat messages in the thread.
    let messages = Message::in_thread(&thread.id)
        .order(Message::created_at_desc())
        .limit(25)
        .debug_load(&mut connection)
        .await?;

    // Load the first 10 user subscriptions to the thread.
    let subscriptions = UserSubscription::select()
        .filter(Subscription::by_thread(&thread.id))
        .order(Subscription::created_at_desc())
        .limit(10)
        .debug_load(&mut connection)
        .await?;

    Response::build().json(&ThreadIncludes {
        thread,
        messages,
        subscriptions,
    })
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE)?.thread.id;

    // Deserialize the request body into a thread change set.
    let (body, state) = request.into_future();
    let changes = body.await?.json()?;

    // Acquire a database connection and update the thread.
    let thread = Thread::update(&id, changes)
        .returning(Thread::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    Response::build().json(&thread)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE)?.thread.id;

    // Acquire a database connection.
    let mut connection = request.state().pool().get_owned().await?;

    // Acquire a database connection and delete the thread.
    let rows = Thread::delete(&id).debug_execute(&mut connection).await?;

    // In debug builds assert that the number of affected rows is not zero.
    debug_assert!(rows > 0, "failed to delete thread: {}", &id);

    Response::build().status(204).finish()
}
