use http::StatusCode;
use via::{Next, Request, Response};

use crate::database::models::user::*;
use crate::State;

pub async fn index(request: Request<State>, _: Next<State>) -> via::Result {
    let state = request.state().try_upgrade()?;
    let users = User::all(&state.pool).await?;

    Response::build().json(&users)
}

pub async fn create(request: Request<State>, _: Next<State>) -> via::Result {
    let state = request.state().try_upgrade()?;
    let payload = request.into_body().read_to_end().await?;
    let new_user = payload.parse_json::<NewUser>()?.insert(&state.pool).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_user)
}

pub async fn show(request: Request<State>, _: Next<State>) -> via::Result {
    let id = request.param("id").parse()?;
    let state = request.state().try_upgrade()?;
    let user = User::find(&state.pool, id).await?;

    Response::build().json(&user)
}

pub async fn update(request: Request<State>, _: Next<State>) -> via::Result {
    let id = request.param("id").parse()?;
    let state = request.state().try_upgrade()?;
    let payload = request.into_body().read_to_end().await?;
    let updated_user = payload
        .parse_json::<ChangeSet>()?
        .apply(&state.pool, id)
        .await?;

    Response::build().json(&updated_user)
}

pub async fn destroy(request: Request<State>, _: Next<State>) -> via::Result {
    let id = request.param("id").parse()?;
    let state = request.state().try_upgrade()?;

    User::delete(&state.pool, id).await?;
    Response::build().status(StatusCode::NO_CONTENT).finish()
}
