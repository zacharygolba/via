use diesel::prelude::*;
use via::{Payload, Response};

use crate::models::user::*;
use crate::util::error::forbidden;
use crate::util::{Authenticate, DebugQueryDsl, Page, Paginate, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Get pagination params from the URI query.
    let page = request.envelope().query::<Page>()?;

    // Acquire a database connection and execute the query.
    let users = User::query()
        .select(User::as_select())
        .order(recent())
        .paginate(page)
        .debug_load(&mut request.app().database().await?)
        .await?;

    Response::build().json(&users)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // Deserialize a new user from the request body.
    let (body, app) = request.into_future();
    let new_user = body.await?.json::<NewUser>()?;

    // Acquire a database connection and execute the insert.
    let user = diesel::insert_into(users::table)
        .values(new_user)
        .returning(User::as_returning())
        .debug_result(&mut app.database().await?)
        .await?;

    // Build the response and update the session cookie.
    let mut response = Response::build().status(201).json(&user)?;
    response.set_user(app.secret(), Some(*user.id()))?;

    Ok(response)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("user-id").parse()?;

    // Acquire a database connection and find the user.
    let user = User::query()
        .select(User::as_select())
        .filter(by_id(&id))
        .debug_first(&mut request.app().database().await?)
        .await?;

    Response::build().json(&user)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("user-id").parse()?;

    if id != *request.user()? {
        return forbidden();
    }

    // Deserialize a reaction changeset from the request body.
    let (body, app) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and update the user.
    let user = diesel::update(users::table)
        .filter(by_id(&id))
        .set(changes)
        .returning(User::as_returning())
        .debug_result(&mut app.database().await?)
        .await?;

    Response::build().json(&user)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("user-id").parse()?;

    if id != *request.user()? {
        return forbidden();
    }

    // Acquire a database connection and delete the user.
    diesel::delete(users::table)
        .filter(by_id(&id))
        .debug_execute(&mut request.app().database().await?)
        .await?;

    Response::build().status(204).finish()
}
