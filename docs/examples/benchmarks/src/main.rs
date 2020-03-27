use via::prelude::*;

#[action(GET, "/text")]
async fn text() -> impl Respond {
    "Hello, world!"
}

#[action(GET, "/unit")]
async fn unit() -> impl Respond {}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.mount(text);
    app.mount(unit);
    app.listen(("0.0.0.0", 8080)).await
}