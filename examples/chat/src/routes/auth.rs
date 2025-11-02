use cookie::{Cookie, SameSite};
use via::{Next, Payload, Request, Response};

use crate::models::{Chat, UserParams};

pub async fn login(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let (head, body) = request.into_parts();
    let state = head.into_state();

    let params = body.into_future().await?.serde_json::<UserParams>()?;
    let user = state.insert(params).await?;

    let mut response = Response::build().json(&user)?;

    response.cookies_mut().private_mut(state.secret()).add(
        Cookie::build(("via-chat-session", user.id().to_string()))
            .http_only(true)
            .path("/")
            .same_site(SameSite::Strict)
            .secure(true),
    );

    Ok(response)
}
