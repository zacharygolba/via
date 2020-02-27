use via::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.route(echo);
    via::start(app).await
}

#[via::route("/*path")]
async fn echo(path: String, context: Context) -> String {
    format!("{} {}", context.method(), path)
}
