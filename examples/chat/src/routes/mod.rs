mod chat;

use cookie::{Cookie, SameSite};
use http::header::CONTENT_SECURITY_POLICY;
use via::{Next, Payload, Request, Response, Route};

use crate::chat::{Chat, UserParams};

pub fn chat(router: &mut Route<Chat>) {
    let join = via::websocket(chat::join);
    let index = via::get(chat::index);
    let message = via::get(chat::message);
    let reaction = via::get(chat::reaction);

    router.respond(index);
    router.route("/join").respond(join);
    router.route("/messages/:id").respond(message);
    router.route("/reactions/:id").respond(reaction);
}

pub async fn home(_: Request<Chat>, _: Next<Chat>) -> via::Result {
    const CSP: &str = "default-src 'self'; connect-src 'self'";

    Response::build()
        .header(CONTENT_SECURITY_POLICY, CSP)
        .text("Chat Example frontend coming soon!")
}

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
