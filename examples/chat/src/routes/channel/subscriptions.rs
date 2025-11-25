use diesel::prelude::*;
use via::{Payload, Response};

use super::authorization::{Ability, Subscriber};
use crate::models::subscription::*;
use crate::schema::subscriptions;
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, Id, Page, Paginate};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the channel.
    let channel_id = request.can(AuthClaims::VIEW)?;

    let page = request.envelope().query::<Page>()?;

    // List subscriptions to the channel with id = :channel-id.
    let subscriptions = Subscription::users()
        .select(UserSubscription::as_select())
        .filter(by_channel(channel_id))
        .order(recent())
        .paginate(page)
        .debug_load(&mut request.app().database().await?)
        .await?;

    Response::build().json(&subscriptions)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // The current user can invite other users to the channel.
    let channel_id = request.can(AuthClaims::INVITE).copied()?;
    let (body, app) = request.into_future();

    // Deserialize the request body into a new subscription.
    let mut new_subscription = body.await?.json::<NewSubscription>()?;

    new_subscription.channel_id = Some(channel_id);

    // Acquire a database connection and create the subscription.
    let subscription = diesel::insert_into(subscriptions::table)
        .values(new_subscription)
        .returning(Subscription::as_select())
        .debug_result(&mut app.database().await?)
        .await?;

    Response::build().status(201).json(&subscription)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the channel.
    let channel_id = request.can(AuthClaims::VIEW)?;

    // The id of the subscription.
    let id = request.envelope().param("subscription-id").parse()?;

    // Acquire a database connection and find the subscription.
    let subscription = Subscription::users()
        .select(UserSubscription::as_select())
        .filter(by_id(&id).and(by_channel(channel_id)))
        .debug_first(&mut request.app().database().await?)
        .await?;

    Response::build().json(&subscription)
}

/// Returns the parsed :subscription-id param from the request uri if the
/// subscription is owned by the current user or they have
/// [`AuthClaims::MODERATE`].
///
fn is_owner_or_moderator(request: &Request) -> via::Result<Id> {
    // The current user's subscription to the channel.
    let subscription = request.subscription()?;

    // The id of the subscription that the user wants to mutate.
    let id = request.envelope().param("subscription-id").parse()?;

    if subscription.id() == &id || subscription.can(AuthClaims::MODERATE).is_ok() {
        Ok(id)
    } else {
        forbidden()
    }
}

pub async fn update(request: Request, _: Next) -> via::Result {
    // The current user can update the subscription.
    let id = is_owner_or_moderator(&request)?;

    // Deserialize a subscription change set from the body.
    let (body, app) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and update the subscription.
    let subscription = diesel::update(subscriptions::table)
        .filter(by_id(&id))
        .set(changes)
        .returning(Subscription::as_returning())
        .debug_result(&mut app.database().await?)
        .await?;

    Response::build().json(&subscription)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    // The current user can delete the subscription.
    let id = is_owner_or_moderator(&request)?;

    // Acquire a database connection and delete the subscription.
    diesel::delete(subscriptions::table)
        .filter(by_id(&id))
        .debug_execute(&mut request.app().database().await?)
        .await?;

    Response::build().status(204).finish()
}
