mod api;
mod database;

use std::time::Duration;
use via::error::BoxError;
use via::middleware::Timeout;
use via::{ErrorBoundary, Response, Server};

use database::Pool;

type Request = via::Request<State>;
type Next = via::Next<State>;

struct State {
    pool: Pool,
}

async fn log_request(request: Request, next: Next) -> via::Result<Response> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    next.call(request).await.inspect(|response| {
        let status = response.status();
        // TODO: Replace println with an actual logger.
        println!("{} {} => {}", method, path, status);
    })
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    dotenvy::dotenv()?;

    // Create a new app with our shared state that contains a database pool.
    let mut app = via::new(State {
        pool: database::pool().await?,
    });

    // Setup a simple logger middleware that logs the method, path, and response
    // status code of every request.
    app.include(log_request);

    // Define the /api namespace.
    app.at("/api").scope(|api| {
        use api::{posts, users, util};

        // Redact sensitive information from errors that occur on /api routes.
        // Also configure the error to respond with JSON when converted to a
        // response.
        api.include(ErrorBoundary::map(util::map_error));

        // Configure error reporting for /api routes. We're including this
        // middleware after the ErrorBoundary::map middleware because we want it
        // to run before sensitive information is redacted from the error. This
        // sequence is necessary because we are doing post-processing of the
        // response rather than pre-processing of the request.
        api.include(ErrorBoundary::inspect(util::inspect_error));

        // Add a timeout middleware to the /api routes. This will prevent the
        // server from waiting indefinitely if we lose connection to the
        // database. For this example, we're using a 30 second timeout.
        api.include(Timeout::new(Duration::from_secs(30)));

        // Define the /api/posts resource.
        api.at("/posts").scope(|posts| {
            // A mock authentication middleware that does nothing.
            posts.include(posts::authenticate);

            posts.respond(via::get(posts::index));
            posts.respond(via::post(posts::create));

            posts.at("/:id").scope(|post| {
                post.respond(via::get(posts::show));
                post.respond(via::patch(posts::update));
                post.respond(via::delete(posts::destroy));
            });
        });

        // Define the /api/users resource.
        api.at("/users").scope(|users| {
            users.respond(via::get(users::index));
            users.respond(via::post(users::create));

            users.at("/:id").scope(|user| {
                user.respond(via::get(users::show));
                user.respond(via::patch(users::update));
                user.respond(via::delete(users::destroy));
            });
        });
    });

    // Start the server.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
