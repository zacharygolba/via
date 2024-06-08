use via::prelude::*;

use super::Document;
use crate::database::{models::user::*, Pool};

pub async fn index(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;

    Ok(Document {
        data: User::all(pool).await?,
    })
}

pub async fn create(mut context: Context, _: Next) -> Result<impl IntoResponse> {
    let body: Document<NewUser> = context.read().json().await?;
    let pool = context.get::<Pool>()?;

    Ok(Document {
        data: body.data.insert(&pool).await?,
    })
}

pub async fn show(context: Context, _: Next) -> Result<impl IntoResponse> {
    let id = context.params().get::<i32>("id")?;
    let pool = context.get::<Pool>()?;

    Ok(Document {
        data: User::find(&pool, id).await?,
    })
}

pub async fn update(context: Context, _: Next) -> Result<impl IntoResponse> {
    let id = context.params().get::<i32>("id")?;
    Ok(format!("Update User: {}", id))
}

pub async fn destroy(context: Context, _: Next) -> Result<impl IntoResponse> {
    let id = context.params().get::<i32>("id")?;
    Ok(format!("Destroy User: {}", id))
}
