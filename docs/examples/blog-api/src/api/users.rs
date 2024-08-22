use via::{Response, Result};

use super::{deserialize, Payload};
use crate::database::models::user::*;
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> Result<Response> {
    let state = request.state();
    let users = User::all(&state.pool).await?;

    Response::json(&Payload { data: users }).finish()
}

pub async fn create(request: Request, _: Next) -> Result<Response> {
    let (_, body, state) = request.into_parts();
    let new_user = deserialize::<NewUser>(body).await?;
    let user = new_user.insert(&state.pool).await?;

    Response::json(&Payload { data: user }).finish()
}

pub async fn show(request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();
    let user = User::find(&state.pool, id).await?;

    Response::json(&Payload { data: user }).finish()
}

pub async fn update(request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;
    let (_, body, state) = request.into_parts();
    let change_set = deserialize::<ChangeSet>(body).await?;
    let user = change_set.apply(&state.pool, id).await?;

    Response::json(&Payload { data: user }).finish()
}

pub async fn destroy(request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();

    User::delete(&state.pool, id).await?;
    Response::json(&Payload { data: () }).finish()
}
