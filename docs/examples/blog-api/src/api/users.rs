use via::{Response, Result};

use super::Payload;
use crate::{database::models::user::*, Next, Request};

pub async fn index(request: Request, _: Next) -> Result<Response> {
    let users = User::all(&request.state().pool).await?;

    Response::json(&Payload::new(users)).finish()
}

pub async fn create(mut request: Request, _: Next) -> Result<Response> {
    let body: Payload<NewUser> = request.take_body()?.into_json().await?;
    let user = body.data.insert(&request.state().pool).await?;

    Response::json(&Payload::new(user)).finish()
}

pub async fn show(request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;
    let user = User::find(&request.state().pool, id).await?;

    Response::json(&Payload::new(user)).finish()
}

pub async fn update(mut request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;
    let body: Payload<ChangeSet> = request.take_body()?.into_json().await?;
    let user = body.data.apply(&request.state().pool, id).await?;

    Response::json(&Payload::new(user)).finish()
}

pub async fn destroy(request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;

    User::delete(&request.state().pool, id).await?;
    Response::json(&Payload::new(())).finish()
}
