pub mod auth;
pub mod chat;

use http::header::CONTENT_SECURITY_POLICY;
use via::{Next, Request, Response};

use crate::models::Chat;

pub async fn home(_: Request<Chat>, _: Next<Chat>) -> via::Result {
    const CSP: &str = "default-src 'self'; connect-src 'self'";

    Response::build()
        .header(CONTENT_SECURITY_POLICY, CSP)
        .text("Chat Example frontend coming soon!")
}
