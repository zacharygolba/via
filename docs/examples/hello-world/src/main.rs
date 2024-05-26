use via::prelude::*;

struct Routes;

#[service]
impl Routes {
    includes! {
        |context: Context, next: Next| async {
            let result = next.call(context).await;

            println!("This will be called after the request is processed");
            result
        },
        |context: Context, next: Next| async {
            println!("This will be called before the request is processed");
            next.call(context).await
        },
    }

    #[endpoint(GET, "/hello/:name")]
    async fn hello(name: String) -> Result<impl Respond> {
        Ok(format!("Hello, {}", name))
    }
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
    app.delegate(Routes);

    app.listen(("0.0.0.0", 8080)).await
}
