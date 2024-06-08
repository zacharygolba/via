use via::prelude::*;

use super::Document;
use crate::database::{models::post::*, Pool};

pub async fn authenticate(context: Context, next: Next) -> Result<impl IntoResponse> {
    println!("authenticate");
    next.call(context).await
}

pub async fn index(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;

    Ok(Document {
        data: Post::public(pool).await?,
    })
}

pub async fn create(mut context: Context, _: Next) -> Result<impl IntoResponse> {
    let body: Document<NewPost> = context.read().json().await?;
    let pool = context.get::<Pool>()?;

    Ok(Document {
        data: body.data.insert(pool).await?,
    })
}

pub async fn show(context: Context, _: Next) -> Result<impl IntoResponse> {
    let id = context.params().get::<i32>("id")?;
    let pool = context.get::<Pool>()?;

    Ok(Document {
        data: Post::find(&pool, id).await?,
    })
}

pub async fn update(mut context: Context, _: Next) -> Result<impl IntoResponse> {
    let body: Document<ChangeSet> = context.read().json().await?;
    let pool = context.get::<Pool>()?;
    let id = context.params().get::<i32>("id")?;

    Ok(Document {
        data: body.data.apply(pool, id).await?,
    })
}

pub async fn destroy(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;
    let id = context.params().get::<i32>("id")?;

    Ok(Document {
        data: Post::delete(pool, id).await?,
    })
}
