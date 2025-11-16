use diesel::prelude::*;
use via::{Finalize, Payload, Response};

use super::authorization::Subscriber;
use crate::chat::{Event, EventContext};
use crate::models::reaction::{NewReaction, Reaction, ReactionIncludes};
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, PageAndLimit, Paginate, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    let message_id = request.envelope().param("message-id").parse()?;

    // Get pagination params from the URI query.
    let page = request.envelope().query::<PageAndLimit>()?;

    // Load the reactions for the message with id = :message-id.
    let reactions: Vec<ReactionIncludes> = {
        let mut connection = request.state().pool().get().await?;

        Reaction::to_message(&message_id)
            .order(Reaction::created_at_desc())
            .paginate(page)
            .debug_load(&mut connection)
            .await?
    };

    Response::build().json(&reactions)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (current_user_id, thread_id) = request.subscription()?.foreign_keys();
    let message_id = request.envelope().param("message-id").parse()?;

    // Deserialize a new reaction from the request body.
    let (body, state) = request.into_future();
    let mut new_reaction = body.await?.json::<NewReaction>()?;

    // Source foreign keys from request metadata when possible.
    new_reaction.message_id = Some(message_id);
    new_reaction.user_id = Some(current_user_id);

    // Acquire a database connection and create the reaction.
    let reaction = Reaction::create(new_reaction)
        .returning(Reaction::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    let event = Event::Reaction(reaction);
    let context = EventContext::new(Some(thread_id), current_user_id);
    let response = Response::build().status(201);

    // Notify subscribers that a reaction has been created and respond.
    state.publish(context, event)?.finalize(response)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // Parse a uuid from the reaction-id param in the URI path.
    let id = request.envelope().param("reaction-id").parse()?;

    // Acquire a database connection and find the reaction by id.
    let reaction: ReactionIncludes = Reaction::includes()
        .filter(Reaction::by_id(&id))
        .debug_first(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&reaction)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let current_user_id = *request.user()?;
    let id = request.envelope().param("reaction-id").parse()?;

    // Deserialize a reaction changeset from the request body.
    let (body, state) = request.into_future();
    let changes = body.await?.json()?;

    // Acquire a database connection and update the reaction.
    let Some(reaction) = Reaction::update(&id, changes)
        // The reaction is owned by the current user.
        .filter(Reaction::by_user_id(&current_user_id))
        .returning(Reaction::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await
        .optional()?
    else {
        return forbidden();
    };

    Response::build().json(&reaction)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let current_user_id = *request.user()?;
    let id = request.envelope().param("reaction-id").parse()?;

    // Acquire a database connection and delete the reaction.
    let 1.. = Reaction::delete(&id)
        // The reaction is owned by the current user.
        .filter(Reaction::by_user_id(&current_user_id))
        .debug_execute(&mut request.state().pool().get().await?)
        .await?
    else {
        return forbidden();
    };

    Response::build().status(204).finish()
}
