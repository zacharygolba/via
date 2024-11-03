use std::process::ExitCode;
use via::{BoxError, Next, Request, Server};

async fn hello(request: Request, _: Next) -> via::Result<String> {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    // Create a new application.
    let mut app = via::new(());

    // Define a route that listens on /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    // Start the server.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
