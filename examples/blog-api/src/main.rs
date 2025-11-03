mod database;
mod routes;
mod util;

use std::process::ExitCode;
use via::error::{Error, Rescue};
use via::{App, Server, Timeout};

use database::Pool;

type Request = via::Request<BlogApi>;
type Next = via::Next<BlogApi>;

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
    app.uses(async |request: Request, next: Next| {
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
    api.uses(Rescue::with(util::error_sanitizer));

    // Add a timeout middleware to the /api routes. This will prevent the
    // server from waiting indefinitely if we lose connection to the
    // database. For this example, we're using a 10 second timeout.
    api.uses(Timeout::from_secs(10).or_service_unavailable());

    // Define the /api/posts resource.
    {
        use routes::posts;

        let mut resource = api.route("/posts");
        let (collection, member) = via::rest!(posts);

        resource.uses(posts::authentication);

        resource.route("/").to(collection);
        resource.route("/:id").to(member);
    }

    // Define the /api/users resource.
    {
        use routes::users;

        let mut resource = api.route("/users");
        let (collection, member) = via::rest!(users);

        resource.route("/").to(collection);
        resource.route("/:id").to(member);
    }

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
