use std::process::ExitCode;
use via::middleware::error_boundary;
use via::{App, Next, Request, Response, Server};

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name")?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    // Create a new application.
    let mut app = App::new();

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::map(|error| {
        eprintln!("error: {}", error);
        error.use_canonical_reason()
    }));

    // Define a route that listens on /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
