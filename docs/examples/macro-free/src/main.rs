use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.include(logger);

    app.at("/hello/:name").get(|context: Context, _| async move {
        let name = context.params().get::<String>("name")?;
        Ok::<_, Error>(format!("Hello, {}", name))
    });

    app.listen(("0.0.0.0", 8080)).await
}

async fn logger(context: Context, next: Next) -> Result<impl Respond> {
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
