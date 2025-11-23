use diesel::prelude::*;
use via::{Finalize, Payload, Response};

use super::authorization::Ability;
use crate::chat::{Event, EventContext};
use crate::models::reaction::*;
use crate::models::subscription::AuthClaims;
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, Page, Paginate, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    let message_id = request.envelope().param("message-id").parse()?;

    // Get pagination params from the URI query.
    let page = request.envelope().query::<Page>()?;

    // Load the reactions for the message with id = :message-id.
    let reactions = Reaction::with_user()
        .select(ReactionWithUser::as_select())
        .filter(by_message(&message_id))
        .order(recent())
        .paginate(page)
        .debug_load(&mut request.app().pool().get().await?)
        .await?;

    Response::build().json(&reactions)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let user_id = *request.user()?;
    let thread_id = *request.can(AuthClaims::REACT)?;
    let message_id = request.envelope().param("message-id").parse()?;

    // Deserialize a new reaction from the request body.
    let (body, app) = request.into_future();
    let mut new_reaction = body.await?.json::<NewReaction>()?;

    // Source foreign keys from request metadata when possible.
    new_reaction.message_id = Some(message_id);
    new_reaction.user_id = Some(user_id);

    // Acquire a database connection and create the reaction.
    let reaction = diesel::insert_into(reactions::table)
        .values(new_reaction)
        .returning(Reaction::as_returning())
        .debug_result(&mut app.pool().get().await?)
        .await?;

    let event = Event::Reaction(reaction);
    let context = EventContext::new(Some(thread_id), user_id);
    let response = Response::build().status(201);

    // Notify subscribers that a reaction has been created and respond.
    app.publish(context, event)?.finalize(response)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // Parse a uuid from the reaction-id param in the URI path.
    let id = request.envelope().param("reaction-id").parse()?;

    // Acquire a database connection and find the reaction by id.
    let reaction = Reaction::with_user()
        .select(ReactionWithUser::as_select())
        .filter(by_id(&id))
        .debug_first(&mut request.app().pool().get().await?)
        .await?;

    Response::build().json(&reaction)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let user_id = request.user().cloned()?;
    let id = request.envelope().param("reaction-id").parse()?;

    // Deserialize a reaction changeset from the request body.
    let (body, app) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and update the reaction.
    let Some(reaction) = diesel::update(reactions::table)
        // The reaction belongs to the current user.
        .filter(by_id(&id).and(by_user(&user_id)))
        .set(changes)
        .returning(Reaction::as_returning())
        .debug_result(&mut app.pool().get().await?)
        .await
        .optional()?
    else {
        return forbidden();
    };

    Response::build().json(&reaction)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let user_id = request.user().cloned()?;
    let id = request.envelope().param("reaction-id").parse()?;

    // Acquire a database connection and delete the reaction.
    let 1.. = diesel::delete(reactions::table)
        // The reaction belongs to the current user.
        .filter(by_id(&id).and(by_user(&user_id)))
        .debug_execute(&mut request.app().pool().get().await?)
        .await?
    else {
        return forbidden();
    };

    Response::build().status(204).finish()
}
