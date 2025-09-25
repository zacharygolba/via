use via::{Next, Request, Response};

use crate::BlogApi;
use crate::database::models::post::*;

pub async fn auth(request: Request<BlogApi>, next: Next<BlogApi>) -> via::Result {
    println!("authenticate");
    next.call(request).await
}

pub async fn index(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state();
    let posts = Post::public(&state.pool).await?;

    Response::build().json(&posts)
}

pub async fn create(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let (head, body) = request.into_parts();
    let state = head.state();

    let new_post = body.into_future().await?.parse_json::<NewPost>()?;
    let post = new_post.insert(&state.pool).await?;

    Response::build().status(201).json(&post)
}

pub async fn show(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state();
    let id = request.param("id").parse()?;

    let post = Post::find(&state.pool, id).await?;

    Response::build().json(&post)
}

pub async fn update(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let (head, body) = request.into_parts();
    let state = head.state();
    let id = head.param("id").parse()?;

    let change_set = body.into_future().await?.parse_json::<ChangeSet>()?;
    let post = change_set.apply(&state.pool, id).await?;

    Response::build().json(&post)
}

pub async fn destroy(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state();
    let id = request.param("id").parse()?;

    Post::delete(&state.pool, id).await?;
    Response::build().status(204).finish()
}
