mod database;
mod routes;
mod util;

use std::process::ExitCode;
use via::Server;
use via::error::{Error, Rescue};

use database::Pool;

type Request = via::Request<BlogApi>;
type Next = via::Next<BlogApi>;

struct BlogApi {
    pool: Pool,
}

impl BlogApi {
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    dotenvy::dotenv()?;

    let mut app = via::app(BlogApi {
        pool: database::pool().await?,
    });

    // Setup a simple logger middleware that logs the method, path, and response
    // status code of every request.
    app.uses(async |request: Request, next: Next| {
        let head = request.envelope();

        let method = head.method().clone();
        let path = head.uri().path().to_owned();

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

    // Define the /api/posts resource.
    api.route("/posts").scope(|posts| {
        use routes::posts::authorization;

        let (collection, member) = via::resources!(routes::posts);
        let comments = via::get(async |_, _| todo!());

        posts.uses(authorization);

        posts.route("/").to(collection);
        posts.route("/:id").to(member).scope(|post| {
            post.route("/comments").to(comments);
        });
    });

    // Define the /api/users resource.
    api.route("/users").scope(|users| {
        let (collection, member) = via::resources!(routes::users);

        users.route("/").to(collection);
        users.route("/:id").to(member);
    });

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
