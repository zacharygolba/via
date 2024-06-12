#[macro_use]
extern crate diesel;

mod api;
mod database;

use via::prelude::*;

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
                    let status = response.status().as_u16();
                    (response, status)
                }
                Err(error) => {
                    let response = error.into_response()?;
                    let status = response.status().as_u16();
                    (response, status)
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
        next.call(context).await.map_err(|error| error.json())
    });

    api.at("/posts").scope(|posts| {
        posts.include(api::posts::authenticate);

        posts.respond(via::get(api::posts::index));
        posts.respond(via::post(api::posts::create));

        posts.at("/:id").scope(|post| {
            post.respond(via::get(api::posts::show));
            post.respond(via::patch(api::posts::update));
            post.respond(via::delete(api::posts::destroy));
        });
    });

    api.at("/users").scope(|users| {
        users.respond(via::get(api::users::index));
        users.respond(via::post(api::users::create));

        users.at("/:id").scope(|user| {
            user.respond(via::get(api::users::show));
            user.respond(via::patch(api::users::update));
            user.respond(via::delete(api::users::destroy));
        });
    });

    app.listen(("127.0.0.1", 8080)).await
}
