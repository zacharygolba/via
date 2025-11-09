use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use via::{Finalize, Payload, Response};

use crate::chat::{Event, EventContext};
use crate::models::message::*;
use crate::util::{Authenticate, Cursor, FoundOrForbidden};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Preconditions
    let thread_id = request.envelope().param("thread-id").parse()?;
    let cursor = request.envelope().query::<Cursor>()?;

    // Build the query from URI params.
    let query = Message::query()
        .filter(by_thread(thread_id).and(by_cursor(cursor)))
        .order(created_at_desc())
        .limit(25);

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let messages: Vec<MessageWithJoins> = {
        let pool = request.state().pool();
        let mut conn = pool.get().await?;

        query.load(&mut conn).await?
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

    // Build the insert statement with the params from the body.
    let insert = diesel::insert_into(Message::TABLE)
        .values(params)
        .returning(Message::as_returning());

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&insert));
    }

    // Acquire a database connection and execute the insert.
    let message = {
        let mut conn = state.pool().get().await?;
        insert.get_result(&mut conn).await?
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

    // Build the query from URI params.
    let query = Message::query().filter(by_id(id));

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let message: MessageWithJoins = {
        let mut conn = request.state().pool().get().await?;
        query.first(&mut conn).await?
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

    // Build the update statement with the params from the body.
    let update = diesel::update(Message::TABLE)
        .set(change_set)
        // Proceed if the message is authored by the current user.
        .filter(by_id(message_id).and(by_author(current_user_id)))
        .returning(Message::as_returning());

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&update));
    }

    // Acquire a database connection and execute the update.
    let message = {
        let mut conn = state.pool().get().await?;
        update.get_result(&mut conn).await.found_or_forbidden()?
    };

    Response::build().json(&message)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.envelope().current_user()?.id;
    let message_id = request.envelope().param("message-id").parse()?;

    // Build the delete statement from URI params.
    let delete = diesel::delete(Message::TABLE).filter(
        // Proceed if the message is authored by the current user.
        by_id(message_id).and(by_author(current_user_id)),
    );

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&delete));
    }

    // Acquire a database connection and execute the delete.
    {
        let pool = request.state().pool();
        let mut conn = pool.get().await?;

        delete.execute(&mut conn).await.found_or_forbidden()?;
    }

    Response::build().status(204).finish()
}
