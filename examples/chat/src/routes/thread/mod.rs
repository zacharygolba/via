pub mod messages;
pub mod reactions;
pub mod subscriptions;

pub use authorization::authorization;

mod authorization;

use diesel::prelude::*;
use via::{Payload, Response};

use self::authorization::{Ability, Subscriber};
use crate::models::message::{self, Message, MessageWithAuthor, group_by_message};
use crate::models::reaction::Reaction;
use crate::models::subscription::{self, AuthClaims, Subscription};
use crate::models::thread::*;
use crate::models::user::UserPreview;
use crate::util::DebugQueryDsl;
use crate::util::paginate::PER_PAGE;
use crate::{Next, Request};

const MAX_FACE_STACK_SIZE: i64 = 10;

pub async fn show(request: Request, _: Next) -> via::Result {
    // Clone the thread that we eagerly loaded during authorization.
    let thread = request.subscription()?.thread().clone();

    // Acquire a database connection.
    let mut connection = request.state().pool().get().await?;

    // Load the enough user subscriptions to make a face stack.
    let users = Subscription::users()
        .select(UserPreview::as_select())
        .filter(subscription::by_thread(thread.id()))
        .order(subscription::recent())
        .limit(MAX_FACE_STACK_SIZE)
        .debug_load(&mut connection)
        .await?;

    // Load the first page of recent messages in the thread.
    let messages = Message::with_author()
        .select(MessageWithAuthor::as_select())
        .filter(message::by_thread(thread.id()))
        .order(message::recent())
        .limit(PER_PAGE)
        .debug_load(&mut connection)
        .await?;

    // Load the reactions for each message in the first page.
    let reactions = {
        let ids = messages.iter().map(Identifiable::id);
        Reaction::to_messages(&mut connection, ids).await?
    };

    let thread = thread.joins(users, group_by_message(messages, reactions));

    Response::build().json(&thread)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE).cloned()?;

    // Deserialize the request body into a thread change set.
    let (body, state) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and update the thread.
    let thread = diesel::update(threads::table)
        .filter(by_id(&id))
        .set(changes)
        .returning(Thread::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    Response::build().json(&thread)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE)?;

    // Acquire a database connection and delete the thread.
    diesel::delete(threads::table)
        .filter(by_id(id))
        .debug_execute(&mut request.state().pool().get().await?)
        .await?;

    Response::build().status(204).finish()
}
