mod api;
mod database;

use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Next, Request, Server, Timeout};

use api::{posts, users, util};
use database::Pool;

struct BlogApi {
    pool: Pool,
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    dotenvy::dotenv()?;

    let mut app = App::new(BlogApi {
        pool: database::pool().await?,
    });

    // Setup a simple logger middleware that logs the method, path, and response
    // status code of every request.
    app.middleware(async |request: Request<BlogApi>, next: Next<BlogApi>| {
        let method = request.method().clone();
        let path = request.uri().path().to_owned();

        next.call(request).await.inspect(|response| {
            // TODO: Replace println with an actual logger.
            println!("{} {} => {}", method, path, response.status());
        })
    });

    // Define the /api namespace.
    let mut api = app.route("/api");

    // Capture errors that occur in the api namespace, log them, and then
    // convert them into json responses. Upstream middleware remains
    // unaffected and continues execution.
    api.middleware(Rescue::with(util::error_sanitizer));

    // Add a timeout middleware to the /api routes. This will prevent the
    // server from waiting indefinitely if we lose connection to the
    // database. For this example, we're using a 10 second timeout.
    api.middleware(Timeout::from_secs(10).or_service_unavailable());

    // Define the /api/posts resource.
    api.route("/posts").scope(|resource| {
        // A mock authentication middleware that does nothing.
        resource.middleware(posts::auth);

        resource.to(via::get(posts::index).post(posts::create));
        resource.route("/:id").to(via::get(posts::show)
            .patch(posts::update)
            .delete(posts::destroy));
    });

    // Define the /api/users resource.
    api.route("/users").scope(|resource| {
        resource.to(via::get(users::index).post(users::create));
        resource.route("/:id").to(via::get(users::show)
            .patch(users::update)
            .delete(users::destroy));
    });

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
