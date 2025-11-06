use http::StatusCode;
use via::{Payload, Response};

use crate::database::models::post::*;
use crate::{Next, Request};

pub async fn authorization(request: Request, next: Next) -> via::Result {
    if cfg!(debug_assertions) {
        println!("👍 user can perform the request operation");
    }

    next.call(request).await
}

pub async fn index(request: Request, _: Next) -> via::Result {
    let posts = Post::public(request.head().state().pool()).await?;
    Response::build().json(&posts)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (head, future) = request.into_future();
    let post_params = future.await?.serde_json::<NewPost>()?;

    let new_post = post_params.insert(head.state().pool()).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_post)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let head = request.head();
    let id = head.param("id").parse()?;

    let post_by_id = Post::find(head.state().pool(), id).await?;

    Response::build().json(&post_by_id)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.head().param("id").parse()?;

    let (head, future) = request.into_future();
    let change_set = future.await?.serde_json::<ChangeSet>()?;

    let updated_post = change_set.apply(&head.state().pool, id).await?;

    Response::build().json(&updated_post)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let head = request.head();
    let id = head.param("id").parse()?;

    Post::delete(head.state().pool(), id).await?;

    Response::build().status(StatusCode::NO_CONTENT).finish()
}
