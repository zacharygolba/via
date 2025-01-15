mod api;
mod database;

use std::process::ExitCode;
use std::time::Duration;
use via::middleware::{error_boundary, timeout};
use via::{Next, Request, Server};

use database::Pool;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct State {
    pool: Pool,
}

async fn log_request(request: Request<State>, next: Next<State>) -> via::Result {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    next.call(request).await.inspect(|response| {
        let status = response.status();
        // TODO: Replace println with an actual logger.
        println!("{} {} => {}", method, path, status);
    })
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
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

        // Catch any errors that occur in the API namespace and generate a
        // JSON response from a redacted version of the original error.
        api.include(error_boundary::map(|_, error| {
            eprintln!("Error: {}", error); // Placeholder for tracing...
            util::map_error(error)
        }));

        // Add a timeout middleware to the /api routes. This will prevent the
        // server from waiting indefinitely if we lose connection to the
        // database. For this example, we're using a 30 second timeout.
        api.include(timeout(Duration::from_secs(30)));

        // Define the /api/posts resource.
        api.at("/posts").scope(|posts| {
            // A mock authentication middleware that does nothing.
            posts.include(posts::auth);

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
