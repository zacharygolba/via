use via::{Error, ErrorBoundary, Event, Response, Result};
use via_serve_static::serve_static;

pub type Request = via::Request<()>;
pub type Next = via::Next<()>;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app(());

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

    let mut hey = app.at("/hey/:name");

    hey.include(|request: Request, next: Next| async move {
        println!("Called before the request is handled");
        let response = next.call(request).await?;
        println!("Called after the request is handled");
        Ok::<_, Error>(response)
    });

    hey.respond(via::get(|request: Request, _: Next| async move {
        let name = request.param("name").required()?;
        Response::text(format!("Hey, {}! 👋", name)).end()
    }));

    let mut id = app.at("/:id");

    id.respond(via::get(|request: Request, next: Next| async move {
        if let Ok(id) = request.param("id").parse::<i32>() {
            Response::text(format!("ID: {}", id)).end()
        } else {
            next.call(request).await
        }
    }));

    let mut catch_all = app.at("/catch-all/*name");

    catch_all.respond(via::get(|request: Request, _: Next| async move {
        let path = request.param("name").required()?;
        Response::text(format!("Catch-all: {}", path)).end()
    }));

    serve_static(app.at("/*path")).serve("./public")?;

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
