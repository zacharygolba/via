use via::http::StatusCode;
use via::Response;

use super::util::Payload;
use crate::database::models::user::*;
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result<Response> {
    let state = request.state();

    Response::build().json(&Payload {
        data: User::all(&state.pool).await?,
    })
}

pub async fn create(request: Request, _: Next) -> via::Result<Response> {
    let state = request.state().clone();
    let new_user = request
        .into_body()
        .read_to_end()
        .parse_json::<NewUser>()
        .await?;

    Response::build()
        .status(StatusCode::CREATED)
        .json(&Payload {
            data: new_user.insert(&state.pool).await?,
        })
}

pub async fn show(request: Request, _: Next) -> via::Result<Response> {
    let id = request.param("id").parse()?;
    let state = request.state();

    Response::build().json(&Payload {
        data: User::find(&state.pool, id).await?,
    })
}

pub async fn update(request: Request, _: Next) -> via::Result<Response> {
    let id = request.param("id").parse()?;
    let state = request.state().clone();
    let change_set = request
        .into_body()
        .read_to_end()
        .parse_json::<ChangeSet>()
        .await?;

    Response::build().json(&Payload {
        data: change_set.apply(&state.pool, id).await?,
    })
}

pub async fn destroy(request: Request, _: Next) -> via::Result<Response> {
    let id = request.param("id").parse()?;
    let state = request.state();

    User::delete(&state.pool, id).await?;
    Response::build().status(StatusCode::NO_CONTENT).finish()
}
