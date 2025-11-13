use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use via::{Finalize, Payload, Response};

use crate::chat::{Event, EventContext};
use crate::models::message::*;
use crate::util::{Cursor, DebugQueryDsl, FoundOrForbidden, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Preconditions
    let thread_id = request.envelope().param("thread-id").parse()?;
    let cursor = request.envelope().query::<Cursor>()?;

    // Acquire a database connection and execute the query.
    let messages: Vec<MessageWithAuthor> = {
        let mut connection = request.state().pool().get().await?;

        Message::query()
            .filter(by_thread(thread_id).and(by_cursor(cursor)))
            .order(created_at_desc())
            .limit(25)
            .debug_load(&mut connection)
            .await?
    };

    Response::build().json(&messages)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.envelope().current_user()?.id;
    let thread_id = request.envelope().param("thread-id").parse()?;

    // Deserialize the request body into message params.
    let (body, state) = request.into_future();
    let mut params = body.await?.json::<NewMessage>()?;

    // Source foreign keys from request metadata when possible.
    params.author_id = Some(current_user_id);
    params.thread_id = Some(thread_id);

    // Acquire a database connection and execute the insert.
    let message = {
        let mut connection = state.pool().get().await?;

        diesel::insert_into(Message::TABLE)
            .values(params)
            .returning(Message::as_returning())
            .get_result(&mut connection)
            .await?
    };

    let event = Event::Message(message);
    let context = EventContext::new(Some(thread_id), current_user_id);
    let response = Response::build().status(201);

    // Notify subscribers that a message has been created and respond.
    state.publish(context, event)?.finalize(response)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // Preconditions
    let id = request.envelope().param("message-id").parse()?;

    // Acquire a database connection and execute the query.
    let message: MessageWithAuthor = {
        let mut connection = request.state().pool().get().await?;

        Message::query()
            .filter(by_id(id))
            .debug_first(&mut connection)
            .await?
    };

    Response::build().json(&message)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.envelope().current_user()?.id;
    let message_id = request.envelope().param("message-id").parse()?;

    // Deserialize the request body into message params.
    let (body, state) = request.into_future();
    let change_set = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and execute the update.
    let message = {
        let mut connection = state.pool().get().await?;

        diesel::update(Message::TABLE)
            .set(change_set)
            .filter(by_id(message_id).and(by_author(current_user_id)))
            .returning(Message::as_returning())
            .get_result(&mut connection)
            .await
            .found_or_forbidden()?
    };

    Response::build().json(&message)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.envelope().current_user()?.id;
    let message_id = request.envelope().param("message-id").parse()?;

    // Acquire a database connection and execute the delete.
    {
        let mut connection = request.state().pool().get().await?;
        let result = diesel::delete(Message::TABLE)
            .filter(by_id(message_id).and(by_author(current_user_id)))
            .debug_execute(&mut connection)
            .await;

        result.found_or_forbidden()?;
    }

    Response::build().status(204).finish()
}
