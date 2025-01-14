mod tls;

use std::process::ExitCode;
use via::middleware::error_boundary;
use via::{BoxError, Next, Request, Server};

async fn hello(request: Request, _: Next) -> via::Result<String> {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Ok(format!("Hello, {}! (via TLS)", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    // Confirm that our certificate and private key exist and are valid before
    // doing anything else.
    let tls_config = tls::server_config().expect("tls config is invalid or missing");

    // Create a new app by calling the `via::app` function.
    let mut app = via::new(());

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::catch(|error, _| {
        eprintln!("Error: {}", error);
    }));

    // Add our hello responder to the endpoint /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    // Start the server.
    Server::new(app)
        .rustls_config(tls_config)
        .listen(("127.0.0.1", 8080))
        .await
}
