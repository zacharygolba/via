use via::http::StatusCode;
use via::{Error, Response};

use super::{deserialize, Payload};
use crate::database::models::post::*;
use crate::{Next, Request};

pub async fn authenticate(request: Request, next: Next) -> Result<Response, Error> {
    println!("authenticate");
    next.call(request).await
}

pub async fn index(request: Request, _: Next) -> Result<Response, Error> {
    let state = request.state();
    let posts = Post::public(&state.pool).await?;

    Response::json(&Payload { data: posts })
}

pub async fn create(request: Request, _: Next) -> Result<Response, Error> {
    let state = request.state().clone();
    let new_post = deserialize::<NewPost>(request.into_body()).await?;
    let post = new_post.insert(&state.pool).await?;

    Response::build()
        .json(&Payload { data: post })
        .status(StatusCode::CREATED)
        .finish()
}

pub async fn show(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();
    let post = Post::find(&state.pool, id).await?;

    Response::json(&Payload { data: post })
}

pub async fn update(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state().clone();
    let change_set = deserialize::<ChangeSet>(request.into_body()).await?;
    let post = change_set.apply(&state.pool, id).await?;

    Response::json(&Payload { data: post })
}

pub async fn destroy(request: Request, _: Next) -> Result<Response, Error> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();

    Post::delete(&state.pool, id).await?;
    Response::json(&Payload { data: () })
}
