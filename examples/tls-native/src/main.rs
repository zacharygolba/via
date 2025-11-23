use native_tls::Identity;
use std::process::ExitCode;
use std::{env, fs};
use via::{Error, Next, Request, Response, Server};

fn load_pkcs12() -> Result<Identity, Error> {
    let identity = fs::read("localhost.p12").expect("failed to load pkcs#12 file");
    let password = env::var("TLS_PKCS_PASSWORD").expect("missing TLS_PKCS_PASSWORD in env");

    Ok(Identity::from_pkcs12(&identity, &password)?)
}

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.envelope().param("name").decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}! (via TLS)", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    dotenvy::dotenv()?;

    // Make sure that our TLS config is present and valid before we proceed.
    let tls_config = load_pkcs12()?;

    let mut app = via::app(());

    // Add our hello responder to the endpoint /hello/:name.
    app.route("/hello/:name").to(via::get(hello));

    Server::new(app)
        .listen_native_tls(("127.0.0.1", 8080), tls_config)
        .await
}
