use http::StatusCode;
use via::{Next, Payload, Request, Response};

use crate::BlogApi;
use crate::database::models::user::*;

pub async fn index(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let users = User::all(request.head().state().pool()).await?;
    Response::build().json(&users)
}

pub async fn create(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let (head, future) = request.into_future();
    let user_params = future.await?.serde_json::<NewUser>()?;

    let new_user = user_params.insert(head.state().pool()).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_user)
}

pub async fn show(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let head = request.head();
    let id = head.param("id").parse()?;

    let user_by_id = User::find(head.state().pool(), id).await?;

    Response::build().json(&user_by_id)
}

pub async fn update(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let id = request.head().param("id").parse()?;

    let (head, future) = request.into_future();
    let change_set = future.await?.serde_json::<ChangeSet>()?;

    let updated_user = change_set.apply(head.state().pool(), id).await?;

    Response::build().json(&updated_user)
}

pub async fn destroy(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let head = request.head();
    let id = head.param("id").parse()?;

    User::delete(head.state().pool(), id).await?;

    Response::build().status(StatusCode::NO_CONTENT).finish()
}
