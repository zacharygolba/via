mod models;
mod routes;

use std::env;
use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Cookies, Server};

use crate::models::Chat;

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

    app.route("/auth").scope(|auth| {
        use routes::auth::login;

        auth.route("/login").respond(via::post(login));
    });

    app.route("/chat").scope(|chat| {
        use routes::chat::{join, message, reaction, thread};

        chat.route("/join").respond(via::ws(join));
        chat.route("/threads/:id").respond(via::get(thread));
        chat.route("/messages/:id").respond(via::get(message));
        chat.route("/reactions/:id").respond(via::get(reaction));
    });

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
