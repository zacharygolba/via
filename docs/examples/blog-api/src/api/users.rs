use via::prelude::*;

use super::Document;
use crate::database::{models::user::*, Pool};

pub async fn index(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: User::all(pool).await?,
    }))
}

pub async fn create(mut context: Context, _: Next) -> Result<impl IntoResponse> {
    let body: Document<NewUser> = context.body_mut().read_json().await?;
    let pool = context.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: body.data.insert(pool).await?,
    }))
}

pub async fn show(context: Context, _: Next) -> Result<impl IntoResponse> {
    let pool = context.get::<Pool>()?;
    let id = context.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: User::find(pool, id).await?,
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
        data: User::delete(pool, id).await?,
    }))
}
