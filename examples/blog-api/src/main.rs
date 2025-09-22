mod api;
mod database;

use std::process::ExitCode;
use std::time::Duration;
use via::builtin::{rescue, timeout};
use via::{App, BoxError, Next, Request};

use database::Pool;

struct BlogApi {
    pool: Pool,
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    dotenvy::dotenv()?;

    let mut app = App::new(BlogApi {
        pool: database::pool().await?,
    });

    // Setup a simple logger middleware that logs the method, path, and response
    // status code of every request.
    app.include(async |request: Request<BlogApi>, next: Next<BlogApi>| {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        next.call(request).await.inspect(|response| {
            let status = response.status();
            // TODO: Replace println with an actual logger.
            println!("{} {} => {}", method, path, status);
        })
    });

    // Define the /api namespace.
    app.at("/api").scope(|api| {
        use api::{posts, users, util};

        // Capture errors that occur in the api namespace, log them, and then
        // convert them into json responses. Upstream middleware remains
        // unaffected and continues execution.
        api.include(rescue::map(util::map_error));

        // Add a timeout middleware to the /api routes. This will prevent the
        // server from waiting indefinitely if we lose connection to the
        // database. For this example, we're using a 10 second timeout.
        api.include(timeout(Duration::from_secs(10)));

        // Define the /api/posts resource.
        api.at("/posts").scope(|posts| {
            // A mock authentication middleware that does nothing.
            posts.include(posts::auth);

            posts.respond(via::get(posts::index).and(via::post(posts::create)));

            posts.at("/:id").respond(
                via::get(posts::show)
                    .and(via::patch(posts::update))
                    .and(via::delete(posts::destroy)),
            );
        });

        // Define the /api/users resource.
        api.at("/users").scope(|users| {
            users.respond(via::get(users::index).and(via::post(users::create)));

            users.at("/:id").respond(
                via::get(users::show)
                    .and(via::patch(users::update))
                    .and(via::delete(users::destroy)),
            );
        });
    });

    via::serve(app).listen(("127.0.0.1", 8080)).await
}
