use via::{Response, Result};

use super::Payload;
use crate::{database::models::post::*, Next, Request};

pub async fn authenticate(request: Request, next: Next) -> Result<Response> {
    println!("authenticate");
    next.call(request).await
}

pub async fn index(request: Request, _: Next) -> Result<Response> {
    let posts = Post::public(&request.state().pool).await?;

    Response::json(&Payload::new(posts)).finish()
}

pub async fn create(mut request: Request, _: Next) -> Result<Response> {
    let body: Payload<NewPost> = request.body_mut().into_json().await?;
    let post = body.data.insert(&request.state().pool).await?;

    Response::json(&Payload::new(post)).finish()
}

pub async fn show(request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;
    let post = Post::find(&request.state().pool, id).await?;

    Response::json(&Payload::new(post)).finish()
}

pub async fn update(mut request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;
    let body: Payload<ChangeSet> = request.body_mut().into_json().await?;
    let post = body.data.apply(&request.state().pool, id).await?;

    Response::json(&Payload::new(post)).finish()
}

pub async fn destroy(request: Request, _: Next) -> Result<Response> {
    let id = request.param("id").parse::<i32>()?;

    Post::delete(&request.state().pool, id).await?;
    Response::json(&Payload::new(())).finish()
}
