mod api;
mod database;

use via::{http::StatusCode, ErrorBoundary, Event, Result};

use database::Pool;

type Request = via::Request<State>;
type Next = via::Next<State>;

pub struct State {
    pub pool: Pool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let mut app = via::app(State {
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
    app.include(ErrorBoundary::inspect(|error| {
        eprintln!("ERROR: {}", error);
    }));

    let mut api = app.at("/api");

    // Apply specific error handling logic to the /api namespace.
    api.include(ErrorBoundary::map(|mut error| {
        use diesel::result::Error as DieselError;

        if let Some(DieselError::NotFound) = error.source().downcast_ref() {
            // The error occurred because a record was not found in the
            // database, set the status to 404 Not Found.
            *error.status_mut() = StatusCode::NOT_FOUND;
        }

        // Return the error with the response format of JSON.
        error.json()
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

    app.listen(("127.0.0.1", 8080), |event| match event {
        Event::ConnectionError(error) | Event::UncaughtError(error) => {
            eprintln!("Error: {}", error);
        }
        Event::ServerReady(address) => {
            println!("Server listening at http://{}", address);
        }
    })
    .await
}
