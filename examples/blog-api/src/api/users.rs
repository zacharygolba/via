use via::{Next, Request, Response};

use crate::database::models::user::*;
use crate::BlogApi;

pub async fn index(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state();
    let users = User::all(&state.pool).await?;

    Response::build().json(&users)
}

pub async fn create(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let (head, body) = request.into_parts();
    let state = head.state();

    let new_user = body.into_future().await?.parse_json::<NewUser>()?;
    let user = new_user.insert(&state.pool).await?;

    Response::build().status(201).json(&user)
}

pub async fn show(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state();
    let id = request.param("id").parse()?;

    let user = User::find(&state.pool, id).await?;

    Response::build().json(&user)
}

pub async fn update(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let (head, body) = request.into_parts();
    let state = head.state();
    let id = head.param("id").parse()?;

    let change_set = body.into_future().await?.parse_json::<ChangeSet>()?;
    let user = change_set.apply(&state.pool, id).await?;

    Response::build().json(&user)
}

pub async fn destroy(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state();
    let id = request.param("id").parse()?;

    User::delete(&state.pool, id).await?;
    Response::build().status(204).finish()
}
