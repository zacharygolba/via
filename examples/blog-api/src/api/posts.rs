use http::StatusCode;
use via::Response;

use crate::database::models::post::*;
use crate::{Next, Request};

pub async fn auth(request: Request, next: Next) -> via::Result {
    println!("authenticate");
    next.call(request).await
}

pub async fn index(request: Request, _: Next) -> via::Result {
    let posts = Post::public(&request.state().pool).await?;

    Response::build().json(&posts)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let state = request.state().clone();
    let payload = request.into_body().read_to_end().await?;
    let new_post = payload.parse_json::<NewPost>()?.insert(&state.pool).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_post)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let id = request.param("id").parse()?;
    let post = Post::find(&request.state().pool, id).await?;

    Response::build().json(&post)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.param("id").parse()?;
    let state = request.state().clone();
    let payload = request.into_body().read_to_end().await?;
    let updated_post = payload
        .parse_json::<ChangeSet>()?
        .apply(&state.pool, id)
        .await?;

    Response::build().json(&updated_post)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.param("id").parse()?;

    Post::delete(&request.state().pool, id).await?;
    Response::build().status(StatusCode::NO_CONTENT).finish()
}
