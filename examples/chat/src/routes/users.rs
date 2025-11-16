use diesel::prelude::*;
use via::{Payload, Response};

use crate::models::User;
use crate::util::{Authenticate, DebugQueryDsl, PageAndLimit, Paginate};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Get pagination params from the URI query.
    let page = request.envelope().query::<PageAndLimit>()?;

    // Acquire a database connection and execute the query.
    let users = User::select()
        .order(User::created_at_desc())
        .paginate(page)
        .debug_load(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&users)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // Deserialize a new user from the request body.
    let (body, state) = request.into_future();
    let new_user = body.await?.json()?;

    // Acquire a database connection and execute the insert.
    let user = User::create(new_user)
        .returning(User::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    // Build the response and update the session cookie.
    let mut response = Response::build().status(201).json(&user)?;
    response.set_user(state.secret(), Some(user.id))?;

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
