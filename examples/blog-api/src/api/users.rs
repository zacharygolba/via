use http::StatusCode;
use via::{Next, Payload, Request, Response};

use crate::BlogApi;
use crate::database::models::user::*;

pub async fn index(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state().as_ref();
    let users = User::all(&state.pool).await?;

    Response::build().json(&users)
}

pub async fn create(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let (head, body) = request.into_parts();
    let state = head.into_state();

    let user_params = body.into_future().await?.deserialize_json::<NewUser>()?;
    let new_user = user_params.insert(&state.pool).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_user)
}

pub async fn show(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let id = request.param("id").parse()?;
    let state = request.state().as_ref();
    let user_by_id = User::find(&state.pool, id).await?;

    Response::build().json(&user_by_id)
}

pub async fn update(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let id = request.param("id").parse()?;

    let (head, body) = request.into_parts();
    let state = head.into_state();

    let change_set = body.into_future().await?.deserialize_json::<ChangeSet>()?;
    let updated_user = change_set.apply(&state.pool, id).await?;

    Response::build().json(&updated_user)
}

pub async fn destroy(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let id = request.param("id").parse()?;
    let state = request.state().as_ref();

    User::delete(&state.pool, id).await?;
    Response::build().status(StatusCode::NO_CONTENT).finish()
}
