use cookie::{Cookie, SameSite};
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use via::{Payload, Response};

use crate::models::user::*;
use crate::{Chat, Next, Request, SESSION};

fn set_current_user(response: &mut Response, state: &Chat, user: &User) -> via::Result<()> {
    let mut jar = response.cookies_mut().private_mut(state.secret());
    let cookie = Cookie::build((SESSION, serde_json::to_string(&user)?))
        .http_only(true)
        .path("/")
        .same_site(SameSite::Strict)
        .secure(true);

    jar.add(cookie);

    Ok(())
}

pub async fn index(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (body, state) = request.into_future();
    let params = body.await?.json::<NewUser>()?;
    let user = User::create(&mut state.pool().get().await?, params).await?;

    let mut response = Response::build().status(201).json(&user)?;
    set_current_user(&mut response, &state, &user)?;

    Ok(response)
}

pub async fn login(request: Request, _: Next) -> via::Result {
    let (body, state) = request.into_future();
    let params = body.await?.json::<LoginParams>()?;
    let user = User::query()
        .filter(by_username(&params.username))
        .first(&mut state.pool().get().await?)
        .await?;

    let mut response = Response::build().json(&user)?;
    set_current_user(&mut response, &state, &user)?;

    Ok(response)
}

pub async fn logout(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn show(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn update(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn destroy(_: Request, _: Next) -> via::Result {
    todo!()
}
