use via::prelude::*;

#[http(GET, "/")]
async fn hello() -> impl Respond {
    "Hello, world!"
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.service(hello);
    app.listen(("0.0.0.0", 8080)).await
}
