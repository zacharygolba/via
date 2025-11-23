mod tls;

use std::process::ExitCode;
use via::{Error, Next, Request, Response, Server};

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.envelope().param("name").decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}! (via TLS)", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    // Make sure that our TLS config is present and valid before we proceed.
    let tls_config = tls::server_config().expect("tls config is invalid or missing");

    let mut app = via::app(());

    // Add our hello responder to the endpoint /hello/:name.
    app.route("/hello/:name").to(via::get(hello));

    Server::new(app)
        .listen_rustls(("127.0.0.1", 8080), tls_config)
        .await
}
