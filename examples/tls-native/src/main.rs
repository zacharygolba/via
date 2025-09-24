use native_tls::Identity;
use std::process::ExitCode;
use std::{env, fs};
use via::{App, BoxError, Next, Request, Response};

fn load_pkcs12() -> Result<Identity, BoxError> {
    let identity = fs::read("localhost.p12").expect("failed to load pkcs#12 file");
    let password = env::var("TLS_PKCS_PASSWORD").expect("missing TLS_PKCS_PASSWORD in env");

    Ok(Identity::from_pkcs12(&identity, &password)?)
}

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}! (via TLS)", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    dotenvy::dotenv()?;

    let mut app = App::new(());

    // Add our hello responder to the endpoint /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    via::serve(app)
        .tls_config(load_pkcs12()?)
        .listen(("127.0.0.1", 8080))
        .await
}
