mod api;
mod database;

use via::{ErrorBoundary, Event, Next, Request, Result};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let mut app = via::app();

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

    // Errors that occur in middleware or responders nested within the /api
    // namespace will have their responses converted to JSON.
    api.include(ErrorBoundary::map(|error| error.json()));

    // Include a reference to the database pool in `request` for middleware
    // nested within the /api namespace.
    api.include({
        let pool = database::pool().await?;
        move |mut request: Request, next: Next| {
            request.insert(pool.clone());
            next.call(request)
        }
    });

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
