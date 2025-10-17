use std::process::ExitCode;
use via::{App, BoxError, Next, Request, Response, Server};

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    let mut app = App::new(());

    // Define a route that listens on /hello/:name.
    app.route("/hello/:name").respond(via::get(hello));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
