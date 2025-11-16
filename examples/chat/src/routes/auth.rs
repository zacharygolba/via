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
    let user_opt = User::table()
        .select(User::as_select())
        .filter(by_id(request.user()?))
        .debug_first(&mut request.state().pool().get().await?)
        .await
        .optional()?;

    let mut response = Response::build()
        .status(if user_opt.is_some() { 200 } else { 401 })
        .json(user_opt.as_ref())?;

    response.set_user(request.state().secret(), user_opt.map(|user| user.id))?;

    Ok(response)
}

pub async fn login(request: Request, _: Next) -> via::Result {
    let (body, state) = request.into_future();
    let params = body.await?.json::<LoginParams>()?;

    let Some(user) = User::table()
        .select(User::as_select())
        .filter(by_username(&params.username))
        .debug_first(&mut state.pool().get().await?)
        .await
        .optional()?
    else {
        return unauthorized();
    };

    let mut response = Response::build().json(Some(&user))?;
    response.set_user(state.secret(), Some(user.id))?;

    Ok(response)
}

pub async fn logout(request: Request, _: Next) -> via::Result {
    let mut response = Response::build().status(204).finish()?;
    response.set_user(request.state().secret(), None)?;

    Ok(response)
}
