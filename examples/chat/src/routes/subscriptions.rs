use diesel::prelude::*;
use via::{Payload, Response};

use super::threads::subscription_for;
use crate::models::subscription::{AuthClaims, NewSubscription, Subscription, UserSubscription};
use crate::util::auth::access_denied;
use crate::util::{DebugQueryDsl, LimitAndOffset};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Get a reference to the already parsed thread id from the current user's
    // subscription.
    let thread_id = subscription_for(&request)?.thread_id();

    let LimitAndOffset(limit, offset) = request.envelope().query()?;

    // List subscriptions with thread_id = :thread-id.
    let subscriptions: Vec<UserSubscription> = Subscription::join_user()
        .filter(Subscription::by_thread(thread_id))
        .limit(limit)
        .offset(offset)
        .debug_load(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&subscriptions)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // The current user's subscription to the thread.
    let subscription = subscription_for(&request)?;

    // Confirm that the current user can invite other users to the thread.
    if !subscription.claims().contains(AuthClaims::INVITE) {
        return access_denied();
    }

    // Get the already parsed thread id from the current user's subscription.
    let thread_id = *subscription.thread_id();

    // Deserialize the request body into a new subscription.
    let (body, state) = request.into_future();
    let mut new_subscription = body.await?.json::<NewSubscription>()?;

    new_subscription.thread_id = Some(thread_id);

    // Acquire a database connection and perform the insert.
    let subscription = Subscription::create(new_subscription)
        .returning(Subscription::as_select())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    Response::build().status(201).json(&subscription)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let subscription_id = request.envelope().param("subscription-id").parse()?;

    // Find the subscription with id = :subscription-id.
    let subscription: UserSubscription = Subscription::join_user()
        .filter(Subscription::by_id(&subscription_id))
        .debug_first(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&subscription)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    // The current user's subscription to the thread.
    let users_subscription = subscription_for(&request)?;

    // Confirm that the current user can edit the other user's claims.
    if !users_subscription.claims().contains(AuthClaims::MODERATE) {
        return access_denied();
    }

    // The id of the subscription that the current user wants to edit.
    let subscription_id = request.envelope().param("subscription-id").parse()?;

    // Deserialize the request body into a thread change set.
    let (body, state) = request.into_future();
    let change_set = body.await?.json()?;

    // Acquire a database connection and execute the update.
    let subscription = Subscription::update(&subscription_id, change_set)
        .returning(Subscription::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    Response::build().json(&subscription)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    // The id of the subscription that the current user wants to delete.
    let subscription_id = request.envelope().param("subscription-id").parse()?;

    // The current user's subscription to the thread.
    let users_subscription = subscription_for(&request)?;

    // Confirm that the current user either has the permission to remove the
    // user with the subscription id = :subscription-id from the thread or
    // they themselves are trying to leave the thread.
    if users_subscription.id != subscription_id
        || !users_subscription.claims().contains(AuthClaims::MODERATE)
    {
        return access_denied();
    }

    // Acquire a database connection and execute the delete.
    Subscription::delete(&subscription_id)
        .debug_execute(&mut request.state().pool().get().await?)
        .await?;

    Response::build().status(204).finish()
}
