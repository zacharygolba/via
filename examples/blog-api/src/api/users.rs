use http::StatusCode;
use via::{Error, Response};

use crate::database::models::user::*;
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> Result<Response, Error> {
    let users = User::all(&request.state().pool).await?;

    Response::build().json(&users)
}

pub async fn create(request: Request, _: Next) -> Result<Response, Error> {
    let state = request.state().clone();
    let payload = request.into_body().read_to_end().await?;
    let new_user = payload.parse_json::<NewUser>()?.insert(&state.pool).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_user)
}

pub async fn show(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse()?;
    let user = User::find(&request.state().pool, id).await?;

    Response::build().json(&user)
}

pub async fn update(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse()?;
    let state = request.state().clone();
    let payload = request.into_body().read_to_end().await?;
    let updated_user = payload
        .parse_json::<ChangeSet>()?
        .apply(&state.pool, id)
        .await?;

    Response::build().json(&updated_user)
}

pub async fn destroy(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse()?;

    User::delete(&request.state().pool, id).await?;
    Response::build().status(StatusCode::NO_CONTENT).finish()
}
