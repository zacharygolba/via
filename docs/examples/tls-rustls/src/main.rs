mod tls;

use via::{Error, Next, Request, Server};

async fn hello(request: Request, _: Next) -> Result<String, Error> {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").required()?;

    // Send a plain text response with our greeting message.
    Ok(format!("Hello, {}! (via TLS)", name))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Confirm that our certificate and private key exist and are valid before
    // doing anything else.
    let tls_config = tls::server_config().expect("tls config is invalid or missing");

    // Create a new app by calling the `via::app` function.
    let mut app = via::new(());

    // Add our hello responder to the endpoint /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    Server::new(app)
        // Pass tls_config to the server.
        .rustls_config(tls_config)
        .listen(("127.0.0.1", 6443), |address| {
            println!("Server listening at https://{}", address);
        })
        .await
}
