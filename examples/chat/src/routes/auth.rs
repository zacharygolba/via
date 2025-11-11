use diesel::pg::Pg;
use diesel::{OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use via::{Payload, Response};

use crate::models::user::*;
use crate::util::auth::{Authenticate, Session, unauthorized};
use crate::{Next, Request};

pub async fn me(request: Request, _: Next) -> via::Result {
    // Preconditions
    let id = request.envelope().current_user()?.id;

    // Build the query with the current user's id.
    let query = User::query().filter(by_id(id));

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let Some(user) = ({
        let mut conn = request.state().pool().get().await?;
        query.first(&mut conn).await.optional()?
    }) else {
        return unauthorized();
    };

    // Build the response and update the session cookie.
    let mut response = Response::build().json(&user)?;
    response.set_current_user(request.state().secret(), Some(&user))?;

    Ok(response)
}

pub async fn login(request: Request, _: Next) -> via::Result {
    // Deserialize the JSON params in the request body.
    let (body, state) = request.into_future();
    let params = body.await?.json::<LoginParams>()?;

    // Build the query from the params.
    let query = User::query().filter(by_username(&params.username));

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let Some(user) = ({
        let mut conn = state.pool().get().await?;
        query.first(&mut conn).await.optional()?
    }) else {
        return unauthorized();
    };

    // Build the response and update the session cookie.
    let mut response = Response::build().json(&user)?;
    response.set_current_user(state.secret(), Some(&user))?;

    Ok(response)
}

pub async fn logout(request: Request, _: Next) -> via::Result {
    // Assert the request is authenticated.
    request.envelope().current_user()?;

    // Build the response and update the session cookie.
    let mut response = Response::build().status(204).finish()?;
    response.set_current_user(request.state().secret(), None)?;

    Ok(response)
}
