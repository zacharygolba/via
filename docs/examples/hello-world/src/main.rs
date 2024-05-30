use via::prelude::*;
use via_serve_static::ServeStatic;

async fn logger(context: Context, next: Next) -> Result {
    let path = context.uri().path().to_string();
    let method = context.method().clone();

    next.call(context)
        .await
        .inspect(move |response| {
            let status_code = response.status_code();
            println!("{} {} => {}", method, path, status_code);
        })
        .inspect_err(|error| {
            eprintln!("{}", error);
        })
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.include(logger);
    ServeStatic::new(app.at("/*path")).serve("./public")?;

    let mut hello = app.at("/hello/:name");

    hello.include(|context: Context, next: Next| async move {
        println!("Called before the request is handled");
        let response = next.call(context).await?;
        println!("Called after the request is handled");
        Ok::<_, Error>(response)
    });

    hello.get(|context: Context, _: Next| async move {
        let name: String = context.params().get("name")?;
        Ok::<_, Error>(format!("Hello, {}", name))
    });

    app.listen(("0.0.0.0", 8080)).await
}
