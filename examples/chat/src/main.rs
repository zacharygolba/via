mod chat;
mod models;
mod routes;
mod schema;
mod util;

use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Cookies, Guard, Server, rest, ws};

use chat::Chat;
use routes::homepage;
use util::auth::{self, RestoreSession};

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

    app.uses(Cookies::new().allow(auth::SESSION));
    app.uses(RestoreSession::new());

    app.route("/").to(via::get(homepage));

    let mut api = app.route("/api");

    api.uses(Rescue::with(util::error_sanitizer));

    // The /api/auth resource.
    api.route("/auth").scope(|resource| {
        use routes::auth::{login, logout, me};

        // Login does not require authentication.
        resource.route("/").to(via::post(login));

        // Subsequent routes require authentication.
        resource.uses(Guard::new(auth::required));

        resource.route("/").to(via::delete(logout));
        resource.route("/_me").to(via::get(me));
    });

    // Perform a websocket upgrade and start chatting.
    api.route("/chat").to(ws::upgrade(routes::chat));

    // The /api/threads resource.
    api.route("/threads").scope(|resource| {
        use routes::{messages, reactions, subscriptions, threads};

        // Every route in /api/threads requires authentication.
        resource.uses(Guard::new(auth::required));

        // Setup the authorization middleware for routes nested in /:thread-id.
        //
        // This ensures that the authorization middleware runs before the
        // destroy, show, and update functions for threads and each nested
        // resource.
        //
        // If a user tries to perform an action on a thread or one of it's
        // dependencies and they are not subscribed to the thread or have
        // insufficent permission to perform the action, a 403 forbidden
        // response is returned instead of calling the next middleware.
        resource.route("/:thread-id").uses(threads::authorization);

        // Define the CRUD operations for ./threads. Then, bind the thread
        // route entry to `thread` and continue defining nested resources.
        let mut thread = {
            let (collection, member) = rest!(threads);

            resource.route("/").to(collection);
            resource.route("/:thread-id").to(member)
        };

        thread.route("/messages").scope(|resource| {
            let mut message = {
                let (collection, member) = rest!(messages);

                resource.route("/").to(collection);
                resource.route("/:message-id").to(member)
            };

            message.route("/reactions").scope(|resource| {
                let (collection, member) = rest!(reactions);

                resource.route("/").to(collection);
                resource.route("/:reaction-id").to(member);
            });
        });

        thread.route("/subscriptions").scope(|resource| {
            let (collection, member) = rest!(subscriptions);

            resource.route("/").to(collection);
            resource.route("/:subscription-id").to(member);
        });
    });

    // The /api/users resource.
    api.route("/users").scope(|resource| {
        use routes::users::{self, create, index};

        // Define collection routes separately.
        let (_, member) = rest!(users);

        // Signup does not require authentication.
        resource.route("/").to(via::post(create));

        // Subsequent routes require authentication.
        resource.uses(Guard::new(auth::required));

        resource.route("/").to(via::get(index));
        resource.route("/:user-id").to(member);
    });

    // Start listening at http://localhost:8080 for incoming requests.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
