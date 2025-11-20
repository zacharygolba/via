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

    api.route("/auth").scope(|path| {
        use routes::auth::{login, logout, me};

        path.route("/").to(via::post(login).delete(logout));
        path.route("/_me").to(via::get(me));
    });

    api.route("/chat").scope(|path| {
        // Any request to /api/chat requires authentication.
        path.uses(Guard::new(Request::authenticate));

        // Upgrade to a websocket and start chatting.
        path.route("/").to(ws::upgrade(chat));
    });

    api.route("/threads").scope(|path| {
        // Any request to /api/threads requires authentication.
        path.uses(Guard::new(Request::authenticate));

        // Define create and index on /api/threads.
        //
        // These are commonly referred to as "collection" routes because they
        // operate on a collection of a resource.
        path.route("/").to(rest!(threads as collection));

        // Bind `path` to /api/threads/:thread-id.
        let mut path = path.route("/:thread-id");

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
        path.uses(thread::authorization);

        // Define show, update, and destroy on /api/threads/:thread-id.
        //
        // These are commonly referred to as "member" actions because they
        // operate on a single existing resource.
        path.route("/").to(rest!(thread as member));

        // Continue defining the dependencies of a thread.

        path.route("/messages").scope(|messages| {
            let mut messages = {
                let (collection, member) = rest!(thread::messages);

                messages.route("/").to(collection);
                messages.route("/:message-id").to(member)
            };

            messages.route("/reactions").scope(|reactions| {
                let (collection, member) = rest!(thread::reactions);

                reactions.route("/").to(collection);
                reactions.route("/:reaction-id").to(member);
            });
        });

        path.route("/subscriptions").scope(|subscriptions| {
            let (collection, member) = rest!(thread::subscriptions);

            subscriptions.route("/").to(collection);
            subscriptions.route("/:subscription-id").to(member);
        });
    });

    api.route("/users").scope(|path| {
        // Creating an account does not require authentication.
        path.route("/").to(via::post(users::create));

        // Subsequent requests to /api/users requires authentication.
        path.uses(Guard::new(Request::authenticate));

        path.route("/").to(via::get(users::index));
        path.route("/:user-id").to(rest!(users as member));
    });

    // Start listening at http://localhost:8080 for incoming requests.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
