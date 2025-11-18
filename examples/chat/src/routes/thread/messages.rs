use diesel::prelude::*;
use via::{Finalize, Payload, Response};

use super::authorization::{Ability, Subscriber};
use crate::chat::{Event, EventContext};
use crate::models::message::*;
use crate::models::reaction::Reaction;
use crate::models::subscription::AuthClaims;
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, Keyset, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the thread.
    let thread_id = request.can(AuthClaims::participate())?;
    let keyset = request.envelope().query::<Keyset>()?;

    let mut connection = request.state().pool().get().await?;

    let messages = Message::with_author()
        .select(MessageWithAuthor::as_select())
        .filter(by_thread(thread_id).and(keyset.after(by_recent::columns)))
        .order(by_recent())
        .limit(keyset.limit)
        .debug_load(&mut connection)
        .await?;

    let reactions = {
        let ids = messages.iter().map(Identifiable::id).collect();
        Reaction::to_messages(&mut connection, ids).await?
    };

    Response::build().json(&group_by_message(messages, reactions))
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (user_id, thread_id) = request.subscription()?.foreign_keys();

    // Deserialize a new message from the request body.
    let (body, state) = request.into_future();
    let mut new_message = body.await?.json::<NewMessage>()?;

    new_message.author_id = Some(user_id.clone());
    new_message.thread_id = Some(thread_id.clone());

    // Acquire a database connection and create the message.
    let message = diesel::insert_into(messages::table)
        .values(new_message)
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

    let mut connection = request.state().pool().get().await?;

    // Acquire a database connection and execute the query.
    let message = Message::with_author()
        .select(MessageWithAuthor::as_select())
        .filter(by_id(&id))
        .debug_first(&mut connection)
        .await?;

    let reactions = {
        let ids = vec![&id];
        Reaction::to_messages(&mut connection, ids).await?
    };

    Response::build().json(&message.joins(reactions))
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let user_id = request.user().cloned()?;
    let id = request.envelope().param("message-id").parse()?;

    // Deserialize the request body into message params.
    let (body, state) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and execute the update.
    let Some(message) = diesel::update(messages::table)
        .filter(by_id(&id).and(by_author(&user_id)))
        .set(changes)
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

    if let Err(user_id) = request.subscription()?.can(AuthClaims::MODERATE) {
        // The user that made the request is not a moderator.
        // 403 unless they are the author of the message.
        if let ..1 = diesel::delete(messages::table)
            .filter(by_id(&id).and(by_author(&user_id)))
            .debug_execute(&mut connection)
            .await?
        {
            return forbidden();
        }
    } else {
        // The user that made the request is a moderator.
        diesel::delete(messages::table)
            .filter(by_id(&id))
            .debug_execute(&mut connection)
            .await?;
    }

    Response::build().status(204).finish()
}
