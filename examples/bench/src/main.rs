use via::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.route(hello);
    via::start(app).await
}

#[via::get("/")]
async fn hello() -> &'static str {
    "Hello, world!"
}
