use via::prelude::*;

#[http(GET, "/hello/:name")]
async fn hello(name: String) -> impl Respond {
    format!("Hello, {}", name)
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.service(hello);
    app.listen(("0.0.0.0", 8080)).await
}
