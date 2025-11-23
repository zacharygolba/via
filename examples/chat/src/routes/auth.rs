use diesel::prelude::*;
use serde::Deserialize;
use via::{Payload, Response};

use crate::models::user::*;
use crate::util::error::unauthorized;
use crate::util::{Authenticate, DebugQueryDsl, Session};
use crate::{Next, Request};

#[derive(Deserialize)]
struct LoginParams {
    username: String,
}

pub async fn me(request: Request, _: Next) -> via::Result {
    let Some(user) = User::query()
        .select(User::as_select())
        .filter(by_id(request.user()?))
        .debug_first(&mut request.app().pool().get().await?)
        .await
        .optional()?
    else {
        return unauthorized();
    };

    let mut response = Response::build().json(&user)?;
    response.set_user(request.app().secret(), Some(*user.id()))?;

    Ok(response)
}

pub async fn login(request: Request, _: Next) -> via::Result {
    let (body, app) = request.into_future();
    let params = body.await?.json::<LoginParams>()?;

    let Some(user) = User::query()
        .select(User::as_select())
        .filter(by_username(&params.username))
        .debug_first(&mut app.pool().get().await?)
        .await
        .optional()?
    else {
        return unauthorized();
    };

    let mut response = Response::build().json(&user)?;
    response.set_user(app.secret(), Some(*user.id()))?;

    Ok(response)
}

pub async fn logout(request: Request, _: Next) -> via::Result {
    request.user()?;

    let mut response = Response::build().status(204).finish()?;
    response.set_user(request.app().secret(), None)?;

    Ok(response)
}
