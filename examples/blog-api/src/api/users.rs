use http::StatusCode;
use via::{Next, Request, Response};

use crate::database::models::user::*;
use crate::State;

pub async fn index(request: Request<State>, _: Next<State>) -> via::Result {
    let users = User::all(&request.state().pool).await?;

    Response::build().json(&users)
}

pub async fn create(request: Request<State>, _: Next<State>) -> via::Result {
    let state = request.state().clone();
    let payload = request.into_future().await?;
    let new_user = payload.parse_json::<NewUser>()?.insert(&state.pool).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_user)
}

pub async fn show(request: Request<State>, _: Next<State>) -> via::Result {
    let id = request.param("id")?.parse()?;
    let user = User::find(&request.state().clone().pool, id).await?;

    Response::build().json(&user)
}

pub async fn update(request: Request<State>, _: Next<State>) -> via::Result {
    let id = request.param("id")?.parse()?;
    let state = request.state().clone();
    let payload = request.into_future().await?;
    let updated_user = payload
        .parse_json::<ChangeSet>()?
        .apply(&state.pool, id)
        .await?;

    Response::build().json(&updated_user)
}

pub async fn destroy(request: Request<State>, _: Next<State>) -> via::Result {
    let id = request.param("id")?.parse()?;

    User::delete(&request.state().pool, id).await?;
    Response::build().status(StatusCode::NO_CONTENT).finish()
}
