use std::process::ExitCode;
use tokio::io::stderr;
use via::middleware::error_boundary;
use via::{Next, Request, Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

// For the sake of simplifying doctests, we're specifying that we want to
// use the "current_thread" runtime flavor. You'll most likely not want to
// specify a runtime flavor and simpy use #[tokio::main] if your deployment
// target has more than one CPU core.
#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    // Create a new application.
    let mut app = via::app(());

    app.include(|request: Request, next: Next| {
        let response = next.call(request);

        async { Ok(response.await?.map(|body| body.tee(stderr()))) }
    });

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::inspect(|_, error| {
        eprintln!("Error: {}", error);
    }));

    // Define a route that listens on /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    via::start(app).listen(("127.0.0.1", 8080)).await
}
