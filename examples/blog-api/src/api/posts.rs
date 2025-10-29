use http::StatusCode;
use via::{Next, Payload, Request, Response};

use crate::BlogApi;
use crate::database::models::post::*;

pub async fn auth(request: Request<BlogApi>, next: Next<BlogApi>) -> via::Result {
    println!("authenticate");
    next.call(request).await
}

pub async fn index(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let state = request.state().as_ref();
    let posts = Post::public(&state.pool).await?;

    Response::build().json(&posts)
}

pub async fn create(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let (head, body) = request.into_parts();
    let state = head.into_state();

    let post_params = body.into_future().await?.parse_json::<NewPost>()?;
    let new_post = post_params.insert(&state.pool).await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&new_post)
}

pub async fn show(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let id = request.param("id").parse()?;
    let state = request.state().as_ref();
    let post_by_id = Post::find(&state.pool, id).await?;

    Response::build().json(&post_by_id)
}

pub async fn update(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let id = request.param("id").parse()?;

    let (head, body) = request.into_parts();
    let state = head.into_state();

    let change_set = body.into_future().await?.parse_json::<ChangeSet>()?;
    let updated_post = change_set.apply(&state.pool, id).await?;

    Response::build().json(&updated_post)
}

pub async fn destroy(request: Request<BlogApi>, _: Next<BlogApi>) -> via::Result {
    let id = request.param("id").parse()?;

    let state = request.state().as_ref();
    Post::delete(&state.pool, id).await?;

    Response::build().status(StatusCode::NO_CONTENT).finish()
}
