use via::{IntoResponse, Next, Request, Response, Result};

use super::Document;
use crate::database::{models::post::*, Pool};

pub async fn authenticate(request: Request, next: Next) -> Result<impl IntoResponse> {
    println!("authenticate");
    next.call(request).await
}

pub async fn index(request: Request, _: Next) -> Result<impl IntoResponse> {
    let pool = request.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: Post::public(pool).await?,
    }))
}

pub async fn create(mut request: Request, _: Next) -> Result<impl IntoResponse> {
    let body: Document<NewPost> = request.body_mut().read_json().await?;
    let pool = request.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: body.data.insert(pool).await?,
    }))
}

pub async fn show(request: Request, _: Next) -> Result<impl IntoResponse> {
    let pool = request.get::<Pool>()?;
    let id = request.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: Post::find(&pool, id).await?,
    }))
}

pub async fn update(mut request: Request, _: Next) -> Result<impl IntoResponse> {
    let body: Document<ChangeSet> = request.body_mut().read_json().await?;
    let pool = request.get::<Pool>()?;
    let id = request.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: body.data.apply(pool, id).await?,
    }))
}

pub async fn destroy(request: Request, _: Next) -> Result<impl IntoResponse> {
    let pool = request.get::<Pool>()?;
    let id = request.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: Post::delete(pool, id).await?,
    }))
}
