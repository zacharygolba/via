mod models;
mod routes;

use std::env;
use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Cookies, Server, ws};

use models::Chat;

type Request = via::Request<Chat>;
type Next = via::Next<Chat>;

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    dotenvy::dotenv()?;

    let mut app = App::new(Chat::new(
        env::var("VIA_SECRET_KEY")
            .map(|secret| secret.as_bytes().try_into())
            .expect("missing required env var: VIA_SECRET_KEY")
            .expect("unexpected end of input while parsing VIA_SECRET_KEY"),
    ));

    app.uses(Cookies::new().allow("via-chat-session"));

    app.route("/").to(via::get(routes::home));

    let mut api = app.route("/api");

    api.uses(Rescue::with(|sanitizer| sanitizer.use_json()));

    // Non-RESTful auth routes.
    api.route("/auth").scope(|auth| {
        use routes::auth::{login, logout};

        auth.route("/login").to(via::post(login));
        auth.route("/logout").to(via::delete(logout));
    });

    // Perform a websocket upgrade and start chatting.
    api.route("/chat").to(ws::upgrade(routes::chat));

    // Define the CRUD operations for threads and events.
    api.route("/threads").scope(|threads| {
        let mut thread = {
            let (collection, member) = via::rest!(routes::threads);

            threads.route("/").to(collection);
            threads.route("/:thread-id").to(member)
        };

        thread.route("/events").scope(|events| {
            let (collection, member) = via::rest!(routes::events);

            events.uses(routes::events::authorization);

            events.route("/").to(collection);
            events.route("/:event-id").to(member);
        });
    });

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
