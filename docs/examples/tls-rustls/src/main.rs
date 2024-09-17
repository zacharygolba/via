use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tokio_rustls::rustls;
use via::{Error, Next, Request, Server};

async fn hello(request: Request, _: Next) -> Result<String, Error> {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").required()?;

    // Send a plain text response with our greeting message.
    Ok(format!("Hello (via TLS), {}!\n", name))
}

fn load_certs(path: impl AsRef<Path>) -> Result<Vec<CertificateDer<'static>>, Error> {
    let path = path.as_ref();
    let file = File::open(path).or_else(|_| {
        let message = format!("failed to open cert file at: {:?}", path);
        Err(Error::new(message))
    })?;

    certs(&mut BufReader::new(file))
        .map(|result| result.map_err(|error| error.into()))
        .collect()
}

fn load_key(path: impl AsRef<Path>) -> Result<PrivateKeyDer<'static>, Error> {
    let path = path.as_ref();
    let file = File::open(path).or_else(|_| {
        let message = format!("failed to open key file at: {:?}", path);
        Err(Error::new(message))
    })?;

    private_key(&mut BufReader::new(file))
        .map_err(|error| error.into())
        .and_then(|option| {
            option.ok_or_else(|| {
                let message = "failed to load private key".to_string();
                Error::new(message)
            })
        })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create a new app by calling the `via::app` function.
    let mut app = via::new(());

    // Load the certificate and private key from the file system and use them
    // to create a rustls::ServerConfig.
    let rustls_config = {
        let cert = load_certs("cert.pem")?;
        let key = load_key("key.pem")?;

        rustls::ServerConfig::builder()
            // Disable client authentication to use self-signed certs.
            .with_no_client_auth()
            // We're only using a single cert for this example.
            .with_single_cert(cert, key)
            .map_err(|error| error)?
    };

    // Add our hello responder to the endpoint /hello/:name. Middleware that is
    // added to an endpoint with `.respond()` will only run if a request's path
    // matches the path of the endpoint exactly.
    app.at("/hello/:name").respond(via::get(hello));

    Server::new(app)
        .rustls_config(rustls_config)
        .listen(("127.0.0.1", 9433), |address| {
            println!("Server listening at http://{}", address);
        })
        .await
}
