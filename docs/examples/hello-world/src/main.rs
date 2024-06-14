use via::{Context, Error, Next, Response, Result};
use via_serve_static::ServeStatic;

async fn logger(context: Context, next: Next) -> Result {
    let path = context.uri().path().to_string();
    let method = context.method().clone();

    next.call(context)
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

    hey.include(|context: Context, next: Next| async move {
        println!("Called before the request is handled");
        let response = next.call(context).await?;
        println!("Called after the request is handled");
        Ok::<_, Error>(response)
    });

    hey.respond(via::get(|context: Context, _: Next| async move {
        let name = context.param("name").require()?;
        Response::text(format!("Hey, {}! 👋", name)).end()
    }));

    let mut id = app.at("/:id");

    id.respond(via::get(|context: Context, next: Next| async move {
        if let Ok(id) = context.param("id").parse::<i32>() {
            Response::text(format!("ID: {}", id)).end()
        } else {
            next.call(context).await
        }
    }));

    let mut catch_all = app.at("/catch-all/*name");

    catch_all.respond(via::get(|context: Context, _: Next| async move {
        let path = context.param("name").require()?;
        Response::text(format!("Catch-all: {}", path)).end()
    }));

    ServeStatic::new(app.at("/*path")).serve("./public")?;

    app.listen(("127.0.0.1", 8080)).await
}
