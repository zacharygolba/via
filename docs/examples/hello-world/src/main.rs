use via::prelude::*;

#[http(GET, "/")]
async fn hello() -> impl Respond {
    "Hello, world!"
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();

    app.mount(hello);
    app.listen().await
}
