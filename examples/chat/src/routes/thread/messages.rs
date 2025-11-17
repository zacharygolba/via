use diesel::prelude::*;
use via::{Finalize, Payload, Response};

use super::authorization::{Ability, Subscriber};
use crate::chat::{Event, EventContext};
use crate::models::message::*;
use crate::models::reaction::{Reaction, ReactionPreview};
use crate::models::subscription::AuthClaims;
use crate::models::user::User;
use crate::schema::reactions;
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, Keyset, Paginate, Session};
use crate::{Next, Request};

pub const REACTIONS_PER_MESSAGE: i64 = 6;

pub async fn index(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the thread.
    let thread_id = request.can(AuthClaims::participate())?;
    let keyset = request.envelope().query::<Keyset>()?;

    // Acquire a database connection.
    let mut connection = request.state().pool().get_owned().await?;

    let messages = Message::includes()
        .select(MessageWithAuthor::as_select())
        .filter(by_thread(&thread_id))
        .order(Message::created_at_desc())
        .paginate(keyset.after(Message::as_keyset()))
        .debug_load(&mut connection)
        .await?;

    let reactions = Reaction::belonging_to(&messages)
        .inner_join(User::table())
        .select(ReactionPreview::as_select())
        .distinct_on(reactions::emoji)
        .order((reactions::emoji.asc(), reactions::id.asc()))
        .limit(REACTIONS_PER_MESSAGE)
        .debug_load(&mut connection)
        .await?;

    let messages_with_author_and_reactions: Vec<_> = reactions
        .grouped_by(&messages)
        .into_iter()
        .zip(messages)
        .map(|(reactions, message)| message.joins(reactions))
        .collect();

    Response::build().json(&messages_with_author_and_reactions)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (user_id, thread_id) = request.subscription()?.foreign_keys();

    // Deserialize a new message from the request body.
    let (body, state) = request.into_future();
    let mut new_message = body.await?.json::<NewMessage>()?;

    new_message.author_id = Some(user_id);
    new_message.thread_id = Some(thread_id);

    // Acquire a database connection and create the message.
    let message = Message::create(new_message)
        .returning(Message::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    let event = Event::Message(message);
    let context = EventContext::new(Some(thread_id), user_id);
    let response = Response::build().status(201);

    // Notify subscribers that a message has been created and respond.
    state.publish(context, event)?.finalize(response)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("message-id").parse()?;

    let mut connection = request.state().pool().get_owned().await?;

    // Acquire a database connection and execute the query.
    let message = Message::includes()
        .select(MessageWithAuthor::as_select())
        .filter(by_id(&id))
        .debug_first(&mut connection)
        .await?;

    let reactions = Reaction::belonging_to(&message)
        .inner_join(User::table())
        .select(ReactionPreview::as_select())
        .distinct_on(reactions::emoji)
        .order((reactions::emoji.asc(), reactions::id.asc()))
        .limit(REACTIONS_PER_MESSAGE)
        .debug_load(&mut connection)
        .await?;

    Response::build().json(message.joins(reactions))
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let user_id = *request.user()?;
    let id = request.envelope().param("message-id").parse()?;

    // Deserialize the request body into message params.
    let (body, state) = request.into_future();
    let changes = body.await?.json()?;

    // Acquire a database connection and execute the update.
    let Some(message) = Message::update(&id, changes)
        .filter(by_author(&user_id))
        .returning(Message::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await
        .optional()?
    else {
        return forbidden();
    };

    Response::build().json(&message)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("message-id").parse()?;

    // Acquire a database connection.
    let mut connection = request.state().pool().get().await?;

    if let Err(author_id) = request.subscription()?.can(AuthClaims::MODERATE) {
        // The user that made the request is not a moderator.
        // 403 unless they are the author of the message.
        if let ..1 = Message::delete(&id)
            .filter(by_author(&author_id))
            .debug_execute(&mut connection)
            .await?
        {
            return forbidden();
        }
    } else {
        // The user that made the request is a moderator.
        // Delete by id.
        Message::delete(&id).debug_execute(&mut connection).await?;
    }

    Response::build().status(204).finish()
}
