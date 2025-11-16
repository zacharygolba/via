mod chat;
mod models;
mod routes;
mod schema;

#[macro_use]
mod util;

use http::StatusCode;
use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Cookies, Guard, Server, rest, ws};

use chat::Chat;
use routes::{auth, homepage, threads, users};
use util::session::{self, Session};

use crate::util::Authenticate;

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

    app.uses(Cookies::new().allow(session::COOKIE));

    app.route("/").to(via::get(homepage));

    let mut api = app.route("/api");

    api.uses(async |request: Request, next: Next| {
        let state = request.state().clone();
        let mut response = next.call(request).await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            response.set_user(state.secret(), None)?;
        }

        Ok(response)
    });

    api.uses(Rescue::with(util::error_sanitizer));
    api.uses(session::restore);

    api.route("/auth").scope(|resource| {
        // Unauthenticated users can login.
        resource.route("/").to(via::post(auth::login));

        // Subsequent routes require authentication.
        resource.uses(Guard::new(Request::authenticate));

        resource.route("/").to(via::delete(auth::logout));
        resource.route("/_me").to(via::get(auth::me));
    });

    // Perform a websocket upgrade and start chatting.
    api.route("/chat").scope(|resource| {
        resource.uses(Guard::new(Request::authenticate));
        resource.route("/").to(ws::upgrade(routes::chat));
    });

    api.route("/threads").scope(|resource| {
        // Any operation to threads requires authentication.
        resource.uses(Guard::new(Request::authenticate));

        // If a user tries to perform an action on a thread or one of it's
        // dependencies, they must be the owner of the resource or have
        // sufficent permission to perform the requested operation.
        resource.route("/:thread-id").uses(threads::authorization);

        // Bind `thread` to the router entry at /api/threads/:thread-id.
        let mut thread = {
            let (collection, member) = rest!(threads);

            resource.route("/").to(collection);
            resource.route("/:thread-id").to(member)
        };

        thread.route("/messages").scope(|resource| {
            let mut message = {
                let (collection, member) = rest!(threads::messages);

                resource.route("/").to(collection);
                resource.route("/:message-id").to(member)
            };

            message.route("/reactions").scope(|resource| {
                let (collection, member) = rest!(threads::reactions);

                resource.route("/").to(collection);
                resource.route("/:reaction-id").to(member);
            });
        });

        thread.route("/subscriptions").scope(|resource| {
            let (collection, member) = rest!(threads::subscriptions);

            resource.route("/").to(collection);
            resource.route("/:subscription-id").to(member);
        });
    });

    api.route("/users").scope(|resource| {
        // Unauthenticated users can create an account.
        resource.route("/").to(via::post(users::create));

        // Define collection routes for the users resource separately.
        let (_, member) = rest!(users);

        // Subsequent routes require authentication.
        resource.uses(Guard::new(Request::authenticate));

        resource.route("/").to(via::get(users::index));
        resource.route("/:user-id").to(member);
    });

    // Start listening at http://localhost:8080 for incoming requests.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
