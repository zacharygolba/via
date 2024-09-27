mod api;
mod database;

use diesel::result::Error as DieselError;
use via::http::StatusCode;
use via::{Error, ErrorBoundary, Server};

use database::Pool;

type Request = via::Request<State>;
type Next = via::Next<State>;

struct State {
    pool: Pool,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv()?;

    let mut app = via::new(State {
        pool: database::pool().await?,
    });

    // Setup a simple logger middleware that logs the method, path, and
    // response of each request.
    app.include(|request: Request, next: Next| {
        let method = request.method().clone();
        let path = request.uri().path().to_owned();

        async move {
            next.call(request).await.inspect(|response| {
                println!("{} {} => {}", method, path, response.status());
            })
        }
    });

    // Catch any errors that occur in downstream middleware, convert them
    // into a response and log the error message. Upstream middleware will
    // continue to execute as normal.
    app.include(ErrorBoundary::inspect(|error, _| {
        eprintln!("ERROR: {}", error);
    }));

    let mut api = app.at("/api");

    // Apply specific error handling logic to the /api namespace.
    api.include(ErrorBoundary::map(|error, _| {
        // Define the error argument as a mutable variable.
        let mut error = error;

        match error.source().downcast_ref() {
            // The error occurred because a record was not found in the
            // database, set the status to 404 Not Found.
            Some(DieselError::NotFound) => {
                error.set_status(StatusCode::NOT_FOUND);
            }

            // The error occurred because of a database error. Return a
            // new error with an opaque message.
            Some(_) => {
                let message = "Internal Server Error";
                error = Error::new(message.to_string());
            }

            // The error occurred for some other reason.
            None => {}
        }

        // Configure the error to respond with JSON.
        error.respond_with_json();

        // Return the modified error.
        error
    }));

    api.at("/posts").scope(|posts| {
        use api::posts;

        posts.include(posts::authenticate);

        posts.respond(via::get(posts::index));
        posts.respond(via::post(posts::create));

        posts.at("/:id").scope(|post| {
            post.respond(via::get(posts::show));
            post.respond(via::patch(posts::update));
            post.respond(via::delete(posts::destroy));
        });
    });

    api.at("/users").scope(|users| {
        use api::users;

        users.respond(via::get(users::index));
        users.respond(via::post(users::create));

        users.at("/:id").scope(|user| {
            user.respond(via::get(users::show));
            user.respond(via::patch(users::update));
            user.respond(via::delete(users::destroy));
        });
    });

    // Start the server.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
