use diesel::prelude::*;
use serde::Deserialize;
use via::{Payload, Response};

use crate::models::user::*;
use crate::util::DebugQueryDsl;
use crate::util::auth::{Authenticate, Session, unauthorized};
use crate::{Next, Request};

#[derive(Deserialize)]
struct LoginParams {
    username: String,
}

pub async fn me(request: Request, _: Next) -> via::Result {
    let current_user_id = request.current_user()?.id;
    let current_user = User::select()
        .filter(User::by_id(&current_user_id))
        .debug_first(&mut request.state().pool().get().await?)
        .await
        .optional()?;

    let mut response = Response::build()
        .status(if current_user.is_some() { 200 } else { 401 })
        .json(&current_user)?;

    response.set_current_user(request.state().secret(), current_user.as_ref())?;

    Ok(response)
}

pub async fn login(request: Request, _: Next) -> via::Result {
    let (body, state) = request.into_future();
    let params = body.await?.json::<LoginParams>()?;

    let Some(user) = User::select()
        .filter(User::by_username(&params.username))
        .debug_first(&mut state.pool().get().await?)
        .await
        .optional()?
    else {
        return unauthorized();
    };

    let mut response = Response::build().json(&user)?;
    response.set_current_user(state.secret(), Some(&user))?;

    Ok(response)
}

pub async fn logout(request: Request, _: Next) -> via::Result {
    let mut response = Response::build().status(204).finish()?;
    response.set_current_user(request.state().secret(), None)?;

    Ok(response)
}
