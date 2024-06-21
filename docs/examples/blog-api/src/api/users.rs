use via::{IntoResponse, Next, Request, Response, Result};

use super::Document;
use crate::database::{models::user::*, Pool};

pub async fn index(request: Request, _: Next) -> Result<impl IntoResponse> {
    let pool = request.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: User::all(pool).await?,
    }))
}

pub async fn create(mut request: Request, _: Next) -> Result<impl IntoResponse> {
    let body: Document<NewUser> = request.body_mut().read_json().await?;
    let pool = request.get::<Pool>()?;

    Ok(Response::json(&Document {
        data: body.data.insert(pool).await?,
    }))
}

pub async fn show(request: Request, _: Next) -> Result<impl IntoResponse> {
    let pool = request.get::<Pool>()?;
    let id = request.param("id").parse::<i32>()?;

    Ok(Response::json(&Document {
        data: User::find(pool, id).await?,
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
        data: User::delete(pool, id).await?,
    }))
}
