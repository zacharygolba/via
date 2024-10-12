use via::http::StatusCode;
use via::{Error, Response};

use super::{deserialize, Payload};
use crate::database::models::user::*;
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> Result<Response, Error> {
    let state = request.state();
    let users = User::all(&state.pool).await?;

    Response::json(&Payload { data: users })
}

pub async fn create(request: Request, _: Next) -> Result<Response, Error> {
    let state = request.state().clone();
    let new_user = deserialize::<NewUser>(request.into_body()).await?;
    let user = new_user.insert(&state.pool).await?;

    Response::build()
        .json(&Payload { data: user })
        .status(StatusCode::CREATED)
        .finish()
}

pub async fn show(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();
    let user = User::find(&state.pool, id).await?;

    Response::json(&Payload { data: user })
}

pub async fn update(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state().clone();
    let change_set = deserialize::<ChangeSet>(request.into_body()).await?;
    let user = change_set.apply(&state.pool, id).await?;

    Response::json(&Payload { data: user })
}

pub async fn destroy(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();

    User::delete(&state.pool, id).await?;
    Response::json(&Payload { data: () })
}
