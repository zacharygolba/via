use chrono::NaiveDateTime;
use diesel::prelude::*;
use via::request::QueryParams;
use via::{Finalize, Payload, Response};

use super::authorization::{Ability, Subscriber};
use crate::chat::{Event, EventContext};
use crate::models::message::{Message, MessageIncludes, NewMessage};
use crate::models::subscription::AuthClaims;
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, Session};
use crate::{Next, Request};

struct IndexQuery {
    before: NaiveDateTime,
    limit: i64,
}

pub async fn index(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the thread.
    let thread_id = request.subscription()?.thread_id();
    let query = request.envelope().query::<IndexQuery>()?;

    // Acquire a database connection and execute the query.
    let messages: Vec<MessageIncludes> = Message::in_thread(&thread_id)
        .filter(Message::created_before(&query.before))
        .limit(query.limit)
        .debug_load(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&messages)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (current_user_id, thread_id) = request.subscription()?.foreign_keys();

    // Deserialize a new message from the request body.
    let (body, state) = request.into_future();
    let mut new_message = body.await?.json::<NewMessage>()?;

    new_message.author_id = Some(current_user_id);
    new_message.thread_id = Some(thread_id);

    // Acquire a database connection and create the message.
    let message = Message::create(new_message)
        .returning(Message::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    let event = Event::Message(message);
    let context = EventContext::new(Some(thread_id), current_user_id);
    let response = Response::build().status(201);

    // Notify subscribers that a message has been created and respond.
    state.publish(context, event)?.finalize(response)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("message-id").parse()?;

    // Acquire a database connection and execute the query.
    let message: MessageIncludes = Message::includes()
        .filter(Message::by_id(&id))
        .debug_first(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&message)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let current_user_id = *request.user()?;
    let id = request.envelope().param("message-id").parse()?;

    // Deserialize the request body into message params.
    let (body, state) = request.into_future();
    let changes = body.await?.json()?;

    // Acquire a database connection and execute the update.
    let Some(message) = Message::update(&id, changes)
        .filter(Message::by_author_id(&current_user_id))
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
    let current_user_id = *request.user()?;
    let id = request.envelope().param("message-id").parse()?;

    let 1.. = ({
        // Acquire a database connection.
        let mut connection = request.state().pool().get().await?;

        // Delete the message.
        if request.can(AuthClaims::MODERATE).is_ok() {
            // The current user is a moderator of the thread. Delete by id.
            Message::delete(&id).debug_execute(&mut connection).await?
        } else {
            // The current user must own the message. Otherwise, 403.
            Message::delete(&id)
                .filter(Message::by_author_id(&current_user_id))
                .debug_execute(&mut connection)
                .await?
        }
    }) else {
        return forbidden();
    };

    Response::build().status(204).finish()
}

impl TryFrom<QueryParams<'_>> for IndexQuery {
    type Error = via::Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            before: query.first("before").parse()?,
            limit: query
                .first("limit")
                .into_result()
                .map_or(Ok(25), |param| param.parse())?
                .min(50),
        })
    }
}
