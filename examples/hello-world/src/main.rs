use std::process::ExitCode;
use via::{App, Error, Next, Request, Response, Server};

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.envelope().param("name").decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = App::new(());

    // Define a route that listens on /hello/:name.
    app.route("/hello/:name").to(via::get(hello));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
