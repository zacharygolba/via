pub mod reactions;
pub mod subscriptions;
pub mod threads;

pub use authorization::authorization;

mod authorization;

use diesel::prelude::*;
use via::{Payload, Response};

use crate::models::channel::*;
use crate::models::conversation::{self, Conversation, ConversationDetails, ConversationWithUser};
use crate::models::reaction::Reaction;
use crate::models::subscription::{self, AuthClaims, Subscription};
use crate::models::user::UserPreview;
use crate::schema::channels;
use crate::util::DebugQueryDsl;
use crate::util::paginate::PER_PAGE;
use crate::{Next, Request};
use authorization::{Ability, Subscriber};

const MAX_FACE_STACK_SIZE: i64 = 10;

pub async fn show(request: Request, _: Next) -> via::Result {
    // Clone the channel that we eagerly loaded during authorization.
    let channel = request.subscription()?.channel().clone();

    // Acquire a database connection.
    let mut connection = request.app().database().await?;

    // Load the enough user subscriptions to make a face stack.
    let users = Subscription::users()
        .select(UserPreview::as_select())
        .filter(subscription::by_channel(channel.id()))
        .order(subscription::recent())
        .limit(MAX_FACE_STACK_SIZE)
        .debug_load(&mut connection)
        .await?;

    // Load the first page of recent messages in the channel.
    let messages = Conversation::with_author()
        .select(ConversationWithUser::as_select())
        .filter(conversation::by_channel(channel.id()).and(conversation::is_thread()))
        .order(conversation::recent())
        .limit(PER_PAGE)
        .debug_load(&mut connection)
        .await?;

    // Load the reactions for each message in the first page.
    let reactions = {
        let ids = messages.iter().map(Identifiable::id);
        Reaction::to_conversations(&mut connection, ids).await?
    };

    let channel = channel.joins(users, ConversationDetails::grouped_by(messages, reactions));

    Response::build().json(&channel)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE).cloned()?;

    // Deserialize the request body into a channel change set.
    let (body, app) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and update the channel.
    let channel = diesel::update(channels::table)
        .filter(by_id(&id))
        .set(changes)
        .returning(Channel::as_returning())
        .debug_result(&mut app.database().await?)
        .await?;

    Response::build().json(&channel)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE)?;

    // Acquire a database connection and delete the channel.
    diesel::delete(channels::table)
        .filter(by_id(id))
        .debug_execute(&mut request.app().database().await?)
        .await?;

    Response::build().status(204).finish()
}
