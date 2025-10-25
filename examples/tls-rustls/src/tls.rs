// This module is inspired by the server example in the tokio-rustls repo:
// https://github.com/rustls/tokio-rustls/blob/main/examples/server.rs
//

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tokio_rustls::rustls;
use via::{Error, err};

/// Load the certificate and private key from the file system and use them
/// to create a rustls::ServerConfig.
///
pub fn server_config() -> Result<rustls::ServerConfig, Error> {
    let key = load_key("localhost.key")?;
    let cert = load_certs("localhost.cert")?;
    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert, key)
        .map_err(Box::new)?;

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

fn load_certs(path: impl AsRef<Path>) -> Result<Vec<CertificateDer<'static>>, Error> {
    let mut reader = BufReader::new(File::open(path)?);

    rustls_pemfile::certs(&mut reader)
        .map(|result| result.map_err(|error| error.into()))
        .collect()
}

fn load_key(path: impl AsRef<Path>) -> Result<PrivateKeyDer<'static>, Error> {
    let mut reader = BufReader::new(File::open(path)?);

    rustls_pemfile::private_key(&mut reader)
        .map_err(|error| error.into())
        .and_then(|option| option.ok_or_else(|| err!(message = "failed to load private key")))
}
