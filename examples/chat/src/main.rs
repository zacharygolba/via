mod chat;
mod models;
mod routes;
mod schema;
mod util;

use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Cookies, Guard, Server, rest, ws};

use chat::Chat;
use routes::{chat, homepage, thread, threads, users};
use util::session::{self, Session};

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

    api.uses(Rescue::with(util::error_sanitizer));
    api.uses(session::restore);

    api.route("/auth").scope(|resource| {
        use routes::auth::{login, logout, me};

        resource.route("/").to(via::post(login).delete(logout));
        resource.route("/_me").to(via::get(me));
    });

    api.route("/chat").scope(|resource| {
        // Any request to /api/chat requires authentication.
        resource.uses(Guard::new(Request::authenticate));

        // Upgrade to a websocket and start chatting.
        resource.route("/").to(ws::upgrade(chat));
    });

    api.route("/threads").scope(|resource| {
        // Any request to /api/threads requires authentication.
        resource.uses(Guard::new(Request::authenticate));

        // Define the create and index routes for threads.
        // These are commonly referred to as "collection" routes (`n`).
        resource.route("/").to(rest!(threads as collection));

        // Bind `resource` to /api/threads/:thread-id.
        //
        // We prefer shadowing the variable name `resource` to encourage
        // a linear progression of route definitons.
        let mut resource = resource.route("/:thread-id");

        // If a user tries to perform an action on a thread or one of it's
        // dependencies, they must be the owner of the resource or have
        // sufficent permission to perform the requested action.
        //
        // Including this middleware before anything else in the thread module
        // enforces that the `Ability` and `Subscriber` extension traits are
        // valid as long as they are visible in the type system.
        //
        // This is where seperation of concerns intersects with the uri path
        // and the API contract defined in `routes::thread::authorization`.
        resource.uses(thread::authorization);

        // Define the show, update, and destroy routes for a thread.
        // These are commonly referred to as "member" routes (`1`).
        resource.route("/").to(rest!(thread as member));

        // Continue defining the dependencies of a thread.

        resource.route("/messages").scope(|resource| {
            let mut resource = {
                let (collection, member) = rest!(thread::messages);

                resource.route("/").to(collection);
                resource.route("/:message-id").to(member)
            };

            resource.route("/reactions").scope(|resource| {
                let (collection, member) = rest!(thread::reactions);

                resource.route("/").to(collection);
                resource.route("/:reaction-id").to(member);
            });
        });

        resource.route("/subscriptions").scope(|resource| {
            let (collection, member) = rest!(thread::subscriptions);

            resource.route("/").to(collection);
            resource.route("/:subscription-id").to(member);
        });
    });

    api.route("/users").scope(|resource| {
        // Any request to /api/users requires authentication.
        resource.uses(Guard::new(Request::authenticate));

        let (collection, member) = rest!(users);

        resource.route("/").to(collection);
        resource.route("/:user-id").to(member);
    });

    // Start listening at http://localhost:8080 for incoming requests.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
