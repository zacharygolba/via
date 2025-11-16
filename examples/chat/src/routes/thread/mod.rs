pub mod messages;
pub mod reactions;
pub mod subscriptions;

pub use authorization::authorization;

mod authorization;

use diesel::prelude::*;
use via::{Payload, Response};

use self::authorization::{Ability, Subscriber};
use crate::models::message::{self, Message, MessageIncludes};
use crate::models::subscription::*;
use crate::models::thread::{Thread, ThreadIncludes};
use crate::util::DebugQueryDsl;
use crate::util::paginate::PER_PAGE;
use crate::{Next, Request};

const MAX_FACE_STACK_SIZE: i64 = 10;

pub async fn show(request: Request, _: Next) -> via::Result {
    // Clone the thread that we eagerly loaded during authorization.
    let thread = request.subscription()?.thread().clone();

    // Acquire a database connection.
    let mut connection = request.state().pool().get_owned().await?;

    // Load the first page of recent messages in the thread.
    let messages = Message::includes()
        .select(MessageIncludes::as_select())
        .filter(message::by_thread(thread.id()))
        .order(message::created_at_desc())
        .limit(PER_PAGE)
        .debug_load(&mut connection)
        .await?;

    // Load the enough user subscriptions to make a face stack.
    let subscriptions = Subscription::users()
        .select(UserSubscription::as_select())
        .filter(by_thread(thread.id()))
        .order(created_at_desc())
        .limit(MAX_FACE_STACK_SIZE)
        .debug_load(&mut connection)
        .await?;

    Response::build().json(&ThreadIncludes {
        thread,
        messages,
        subscriptions,
    })
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE)?;

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
    let id = request.can(AuthClaims::MODERATE)?;

    // Acquire a database connection and delete the thread.
    Thread::delete(&id)
        .debug_execute(&mut request.state().pool().get().await?)
        .await?;

    Response::build().status(204).finish()
}
