use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use via::{Payload, Response};

use crate::models::user::{NewUser, User, created_at_desc};
use crate::util::{Authenticate, LimitAndOffset};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Get pagination params from the URI query.
    let LimitAndOffset(limit, offset) = request.envelope().query()?;

    // Build the query from URI params.
    let query = User::query()
        .order(created_at_desc())
        .limit(limit)
        .offset(offset);

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let users = query.load(&mut request.state().pool().get().await?).await?;

    Response::build().json(&users)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (body, state) = request.into_future();
    let params = body.await?.json::<NewUser>()?;

    // Build the insert statement with the params from the body.
    let insert = diesel::insert_into(User::TABLE)
        .values(params)
        .returning(User::as_returning());

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&insert));
    }

    // Acquire a database connection and execute the insert.
    let user = insert.get_result(&mut state.pool().get().await?).await?;

    // Build the response and update the session cookie.
    let mut response = Response::build().status(201).json(&user)?;
    response.set_current_user(state.secret(), Some(&user))?;

    Ok(response)
}

pub async fn show(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn update(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn destroy(_: Request, _: Next) -> via::Result {
    todo!()
}
