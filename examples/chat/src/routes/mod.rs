pub mod messages;
pub mod reactions;
pub mod threads;
pub mod users;

mod subscribe;

pub use subscribe::subscribe;

use http::header::CONTENT_SECURITY_POLICY;
use via::Response;

use crate::{Next, Request};

pub async fn home(_: Request, _: Next) -> via::Result {
    const CSP: &str = "default-src 'self'; connect-src 'self'";

    Response::build()
        .header(CONTENT_SECURITY_POLICY, CSP)
        .text("Chat Example frontend coming soon!")
}
