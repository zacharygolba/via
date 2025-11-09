mod chat;
mod models;
mod routes;
mod schema;
mod util;

use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Cookies, Server, rest, ws};

use chat::Chat;
use util::Auth;

const SESSION: &str = "via-chat-session";

type Request = via::Request<Chat>;
type Next = via::Next<Chat>;

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    dotenvy::dotenv()?;

    let mut app = {
        let pool = chat::establish_pg_connection().await;
        let secret = chat::load_session_secret();

        App::new(Chat::new(pool, secret))
    };

    app.uses(Cookies::new().allow(SESSION));
    app.uses(Auth::new(SESSION));
    app.uses(async |request: Request, next: Next| {
        println!("{:#?}", request);
        next.call(request).await
    });

    app.route("/").to(via::get(routes::home));

    let mut api = app.route("/api");

    api.uses(Rescue::with(util::error_sanitizer));

    api.route("/auth").scope(|auth| {
        use routes::users::{login, logout};

        auth.route("/login").to(via::post(login));
        auth.route("/logout").to(via::post(logout));
    });

    // Perform a websocket upgrade and start chatting.
    api.route("/subscribe").to(ws::upgrade(routes::subscribe));

    api.route("/threads").scope(|threads| {
        let mut thread = {
            let (collection, member) = rest!(routes::threads);

            threads.route("/").to(collection);
            threads.route("/:thread-id").to(member)
        };

        // 403 when the current user is not in the thread.
        // thread.uses(routes::threads::authorization);

        thread.route("/messages").scope(|messages| {
            let mut message = {
                let (collection, member) = rest!(routes::messages);

                messages.route("/").to(collection);
                messages.route("/:message-id").to(member)
            };

            message.route("/reactions").scope(|reactions| {
                let (collection, member) = rest!(routes::reactions);

                reactions.route("/").to(collection);
                reactions.route("/:reaction-id").to(member);
            });
        });

        // Access control is defined by the owner of the thread.
        thread.route("/users").scope(|_| {
            // use routes::threads::{add, remove};
            // users.route("/").to(via::post(add).delete(remove));
        });
    });

    api.route("/users").scope(|users| {
        let (collection, member) = rest!(routes::users);

        users.route("/").to(collection);
        users.route("/:user-id").to(member);
    });

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
