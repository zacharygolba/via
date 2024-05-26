use via::prelude::*;

#[endpoint(GET, "/hello/:name")]
async fn hello(name: String) -> Result<impl Respond> {
    Ok(format!("Hello, {}", name))
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

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.include(logger);
    app.delegate(hello);

    app.listen(("0.0.0.0", 8080)).await
}
