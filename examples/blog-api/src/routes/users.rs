use http::StatusCode;
use via::request::Payload;
use via::response::Response;

use crate::database::models::user::*;
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    let users = User::all(request.state().pool()).await?;
    Response::build().json(&users)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (body, state) = request.into_future();
    let user_params = body.await?.json::<NewUser>()?;

    let new_user = user_params.insert(state.pool()).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_user)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("id").parse()?;
    let user_by_id = User::find(request.state().pool(), id).await?;

    Response::build().json(&user_by_id)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("id").parse()?;

    let (body, state) = request.into_future();
    let change_set = body.await?.json::<ChangeSet>()?;

    let updated_user = change_set.apply(state.pool(), id).await?;

    Response::build().json(&updated_user)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.envelope().param("id").parse()?;

    User::delete(request.state().pool(), id).await?;
    Response::build().status(StatusCode::NO_CONTENT).finish()
}
