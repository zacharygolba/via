use via::{IntoResponse, Response, Result};

use super::Document;
use crate::{database::models::user::*, Next, Request};

pub async fn index(request: Request, _: Next) -> Result<impl IntoResponse> {
    let state = request.state();

    Ok(Response::json(&Document {
        data: User::all(&state.pool).await?,
    }))
}

pub async fn create(mut request: Request, _: Next) -> Result<impl IntoResponse> {
    let body: Document<NewUser> = request.body_mut().read_json().await?;
    let state = request.state();

    Ok(Response::json(&Document {
        data: body.data.insert(&state.pool).await?,
    }))
}

pub async fn show(request: Request, _: Next) -> Result<impl IntoResponse> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();

    Ok(Response::json(&Document {
        data: User::find(&state.pool, id).await?,
    }))
}

pub async fn update(mut request: Request, _: Next) -> Result<impl IntoResponse> {
    let id = request.param("id").parse::<i32>()?;
    let body: Document<ChangeSet> = request.body_mut().read_json().await?;
    let state = request.state();

    Ok(Response::json(&Document {
        data: body.data.apply(&state.pool, id).await?,
    }))
}

pub async fn destroy(request: Request, _: Next) -> Result<impl IntoResponse> {
    let id = request.param("id").parse::<i32>()?;
    let state = request.state();

    Ok(Response::json(&Document {
        data: User::delete(&state.pool, id).await?,
    }))
}
