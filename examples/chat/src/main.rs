mod chat;
mod routes;

use std::env;
use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Cookies, Server};

use crate::chat::Chat;

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    dotenvy::dotenv()?;

    let mut app = App::new(Chat::new(
        env::var("VIA_SECRET_KEY")
            .map(|secret| secret.as_bytes().try_into())
            .expect("missing required env var: VIA_SECRET_KEY")
            .expect("unexpected end of input while parsing VIA_SECRET_KEY"),
    ));

    app.middleware(Cookies::new().allow("via-chat-session"));
    app.middleware(Rescue::with(|sanitizer| sanitizer.use_json()));

    app.route("/").respond(via::get(routes::home));
    app.route("/chat").scope(routes::chat);
    app.route("/login").respond(via::post(routes::login));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
