use via::{Error, Next, Request, Response, Result};
use via_serve_static::ServeStatic;

async fn logger(request: Request, next: Next) -> Result<Response> {
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    next.call(request)
        .await
        .inspect(|response| {
            let status = response.status();
            println!("{} {} => {}", method, path, status);
        })
        .inspect_err(|error| {
            eprintln!("{}", error);
        })
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app();

    app.include(logger);

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

    ServeStatic::new(app.at("/*path")).serve("./public")?;

    app.listen(("127.0.0.1", 8080)).await
}
