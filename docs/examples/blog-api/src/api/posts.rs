use via::{Context, IntoResponse, Next, Response, Result};

use super::Document;
use crate::database::{models::post::*, Pool};

pub async fn authenticate(context: Context, next: Next) -> Result<impl IntoResponse> {
    println!("authenticate");
    next.call(context).await
}

pub async fn index(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: Post::public(pool).await?,
    }))
}

pub async fn create(mut context: Context, _: Next) -> Result<impl IntoResponse> {
    let body: Document<NewPost> = context.body_mut().read_json().await?;
    let pool = context.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: body.data.insert(pool).await?,
    }))
}

pub async fn show(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;
    let id = context.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: Post::find(&pool, id).await?,
    }))
}

pub async fn update(mut context: Context, _: Next) -> Result<impl IntoResponse> {
    let body: Document<ChangeSet> = context.body_mut().read_json().await?;
    let pool = context.get::<Pool>()?;
    let id = context.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: body.data.apply(pool, id).await?,
    }))
}

pub async fn destroy(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;
    let id = context.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: Post::delete(pool, id).await?,
    }))
}
