use via::prelude::*;

#[http(GET, "/text")]
async fn text() -> impl Respond {
    "Hello, world!"
}

#[http(GET, "/unit")]
async fn unit() -> impl Respond {}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.service(text);
    app.service(unit);
    app.listen(("0.0.0.0", 8080)).await
}
