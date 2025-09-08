use std::process::ExitCode;
use via::builtin::rescue;
use via::{BoxError, Next, Request, Response};

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    // Create a new application.
    let mut app = via::app(());

    // Capture errors from downstream, log them, and map them into responses.
    // Upstream middleware remains unaffected and continues execution.
    app.include(rescue::inspect(|error| eprintln!("error: {}", error)));

    // Define a route that listens on /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    via::start(app).listen(("127.0.0.1", 8080)).await
}
