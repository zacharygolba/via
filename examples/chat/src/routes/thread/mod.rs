pub mod messages;
pub mod reactions;
pub mod subscriptions;

pub use authorization::authorization;

mod authorization;

use diesel::prelude::*;
use via::{Payload, Response};

use self::authorization::{Ability, Subscriber};
use self::messages::REACTIONS_PER_MESSAGE;
use crate::models::message::{self, Message, MessageWithAuthor};
use crate::models::reaction::{Reaction, ReactionPreview};
use crate::models::subscription::*;
use crate::models::thread::{Thread, ThreadWithJoins};
use crate::models::user::{User, UserPreview};
use crate::util::DebugQueryDsl;
use crate::util::paginate::PER_PAGE;
use crate::{Next, Request};

const MAX_FACE_STACK_SIZE: i64 = 10;

pub async fn show(request: Request, _: Next) -> via::Result {
    use crate::schema::reactions;

    // Clone the thread that we eagerly loaded during authorization.
    let thread = request.subscription()?.thread().clone();

    // Acquire a database connection.
    let mut connection = request.state().pool().get_owned().await?;

    // Load the enough user subscriptions to make a face stack.
    let users = Subscription::users()
        .select(UserPreview::as_select())
        .filter(by_thread(thread.id()))
        .order(created_at_desc())
        .limit(MAX_FACE_STACK_SIZE)
        .debug_load(&mut connection)
        .await?;

    // Load the first page of recent messages in the thread.
    let messages = Message::includes()
        .select(MessageWithAuthor::as_select())
        .filter(message::by_thread(thread.id()))
        .order(Message::created_at_desc())
        .limit(PER_PAGE)
        .debug_load(&mut connection)
        .await?;

    // Load the reactions for each message in the first page.
    let reactions = Reaction::belonging_to(&messages)
        .inner_join(User::table())
        .select(ReactionPreview::as_select())
        .distinct_on(reactions::emoji)
        .order((reactions::emoji, reactions::id))
        .limit(REACTIONS_PER_MESSAGE)
        .debug_load(&mut connection)
        .await?;

    let joins = ThreadWithJoins {
        users,
        thread,
        messages: reactions
            .grouped_by(&messages)
            .into_iter()
            .zip(messages)
            .map(|(reactions, message)| message.joins(reactions))
            .collect(),
    };

    Response::build().json(&joins)
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
