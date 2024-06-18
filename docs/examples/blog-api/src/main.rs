mod api;
mod database;

use via::{Context, Error, IntoResponse, Next, Result};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let mut app = via::app();
    let pool = database::pool().await?;

    // Setup a simple logger middleware that logs the method, path, and response
    // of each request.
    app.include(|context: Context, next: Next| {
        let method = context.method().clone();
        let path = context.uri().path().to_owned();

        async move {
            let (response, status) = match next.call(context).await {
                Ok(response) => {
                    let status = response.status();
                    (response, status)
                }
                Err(error) => {
                    let status = error.status();
                    (error.into_response().unwrap(), status)
                }
            };

            println!("{} {} => {}", method, path, status);
            Ok::<_, Error>(response)
        }
    });

    let mut api = app.at("/api");

    // Include a reference to the database pool in `context` for each request
    // nested within the /api namespace.
    api.include(move |mut context: Context, next: Next| {
        context.insert(pool.clone());
        next.call(context)
    });

    // Errors that occur in middleware or responders nested within the /api
    // namespace will have there responses converted to JSON.
    api.include(|context: Context, next: Next| async move {
        match next.call(context).await {
            result @ Ok(_) => result,
            Err(error) => Err(error.json()),
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

    app.listen(("127.0.0.1", 8080)).await
}
