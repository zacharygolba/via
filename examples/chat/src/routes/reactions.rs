use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;
use via::Payload;
use via::request::PathParams;
use via::response::{Finalize, Response};

use crate::chat::{Event, EventContext};
use crate::models::reaction::*;
use crate::util::{Authenticate, FoundOrForbidden, LimitAndOffset};
use crate::{Next, Request};

struct ReactionParams {
    thread_id: Uuid,
    message_id: Uuid,
}

impl TryFrom<PathParams<'_>> for ReactionParams {
    type Error = via::Error;

    fn try_from(params: PathParams<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            thread_id: params.get("thread-id").parse()?,
            message_id: params.get("message-id").parse()?,
        })
    }
}

pub async fn index(request: Request, _: Next) -> via::Result {
    // Preconditions
    let message_id = request.head().param("message-id").parse()?;

    // Get pagination params from the URI query.
    let LimitAndOffset(limit, offset) = request.head().query()?;

    // Build the query from URI params.
    let query = Reaction::select()
        .filter(by_message(&message_id))
        .order(created_at_desc())
        .limit(limit)
        .offset(offset);

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let reactions: Vec<ReactionWithJoins> = {
        let pool = request.state().pool();
        let mut conn = pool.get().await?;

        query.load(&mut conn).await?
    };

    Response::build().json(&reactions)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.head().current_user()?.id;
    let params = request.head().params::<ReactionParams>()?;

    // Deserialize the request body into message params.
    let (head, future) = request.into_future();
    let mut new_reaction = future.await?.serde_json::<NewReaction>()?;

    // Source foreign keys from request metadata when possible.
    new_reaction.message_id = Some(params.message_id);
    new_reaction.user_id = Some(current_user_id);

    // Build the insert statement with the params from the body.
    let insert = diesel::insert_into(Reaction::TABLE)
        .values(new_reaction)
        .returning(Reaction::as_returning());

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&insert));
    }

    // Acquire a database connection and execute the insert.
    let reaction = {
        let pool = head.state().pool();
        let mut conn = pool.get().await?;

        insert.get_result(&mut conn).await?
    };

    let context = EventContext::new(Some(params.thread_id), current_user_id);
    let event = Event::Reaction(reaction);

    // Notify subscribers that a reaction has been created and respond.
    head.state()
        .publish(context, event)?
        .finalize(Response::build().status(201))
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // Preconditions
    let id = request.head().param("message-id").parse()?;

    // Build the query from URI params.
    let query = Reaction::select().filter(by_id(&id));

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let reaction: ReactionWithJoins = {
        let pool = request.state().pool();
        let mut conn = pool.get().await?;

        query.first(&mut conn).await?
    };

    Response::build().json(&reaction)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.head().current_user()?.id;
    let reaction_id = request.head().param("reaction-id").parse()?;

    // Deserialize the request body into message params.
    let (head, future) = request.into_future();
    let change_set = future.await?.serde_json::<ChangeSet>()?;

    // Build the update statement with the params from the body.
    let update = diesel::update(Reaction::TABLE)
        .set(change_set)
        // Proceed if the message is authored by the current user.
        .filter(by_id(&reaction_id).and(by_user(&current_user_id)))
        .returning(Reaction::as_returning());

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&update));
    }

    // Acquire a database connection and execute the update.
    let reaction = {
        let pool = head.state().pool();
        let mut conn = pool.get().await?;

        update.get_result(&mut conn).await.found_or_forbidden()?
    };

    Response::build().json(&reaction)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.head().current_user()?.id;
    let reaction_id = request.head().param("reaction-id").parse()?;

    // Build the delete statement from URI params.
    let delete = diesel::delete(Reaction::TABLE).filter(
        // Proceed if the message is authored by the current user.
        by_id(&reaction_id).and(by_user(&current_user_id)),
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
