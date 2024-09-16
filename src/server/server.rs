use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_rustls::rustls;

use super::serve;
use crate::{server::acceptor::HttpAcceptor, App, Error, Router};

/// The default value of the maximum number of concurrent connections.
const DEFAULT_MAX_CONNECTIONS: usize = 256;

/// The default value of the shutdown timeout in seconds.
const DEFAULT_SHUTDOWN_TIMEOUT: u64 = 30;

pub struct Server<T> {
    state: Arc<T>,
    router: Arc<Router<T>>,
    #[cfg(feature = "rustls")]
    key_path: Option<PathBuf>,
    #[cfg(feature = "rustls")]
    cert_path: Option<PathBuf>,
    max_connections: Option<usize>,
    shutdown_timeout: Option<u64>,
}

impl<T: Send + Sync + 'static> Server<T> {
    pub fn new(app: App<T>) -> Self {
        let (state, router) = app.into_parts();

        Self {
            state,
            router: Arc::new(router),
            #[cfg(feature = "rustls")]
            key_path: None,
            #[cfg(feature = "rustls")]
            cert_path: None,
            max_connections: None,
            shutdown_timeout: None,
        }
    }

    #[cfg(feature = "rustls")]
    pub fn key_path(mut self, path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        self.key_path = Some(path.as_ref().to_path_buf());
        self
    }

    #[cfg(feature = "rustls")]
    pub fn cert_path<P>(mut self, path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        self.cert_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets the maximum number of concurrent connections that the server can
    /// accept. The default value is 256.
    ///
    /// We suggest not setting this value unless you know what you are doing and
    /// have a good reason to do so. If you are unsure, it is best to leave this
    /// value at the default and scale horizontally.
    ///
    /// If you do set this value, we suggest doing so by profiling the stack size
    /// of your application when it's under load and incrementally increasing
    /// this value until you find a balance between performance and worry-free
    /// programming. In other words, the closer this value is to the limit based
    /// on your application's stack consumption and the stack size of your server,
    /// the more careful you will need to be when allocating values on the stack
    /// (i.e dereferencing a heap pointer). Otherwise, you may encounter a stack
    /// overflow. In addition to the stack size, you should also consider not
    /// setting this value higher than the number of available file descriptors
    /// (or `ulimit -n`) on POSIX systems.
    ///
    pub fn max_connections(mut self, n: usize) -> Self {
        self.max_connections = Some(n);
        self
    }

    /// Set the amount of time in seconds that the server will wait for inflight
    /// connections to complete before shutting down. The default value is 30
    /// seconds.
    pub fn shutdown_timeout(mut self, timeout: u64) -> Self {
        self.shutdown_timeout = Some(timeout);
        self
    }

    pub async fn listen<A, R>(self, address: A, on_ready: R) -> Result<(), Error>
    where
        A: ToSocketAddrs,
        R: FnOnce(&SocketAddr),
    {
        let max_connections = self.max_connections.unwrap_or(DEFAULT_MAX_CONNECTIONS);
        let shutdown_timeout = self.shutdown_timeout.map_or_else(
            || Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT),
            Duration::from_secs,
        );

        #[cfg(feature = "rustls")]
        let future = listen_rustls(
            self.state,
            self.router,
            self.key_path,
            self.cert_path,
            max_connections,
            shutdown_timeout,
            address,
            on_ready,
        );

        #[cfg(not(feature = "rustls"))]
        let future = listen(
            self.state,
            self.router,
            max_connections,
            shutdown_timeout,
            address,
            on_ready,
        );

        future.await
    }
}

async fn listen<State>(
    state: Arc<State>,
    router: Arc<Router<State>>,
    max_connections: usize,
    shutdown_timeout: Duration,
    address: impl ToSocketAddrs,
    on_ready: impl FnOnce(&SocketAddr),
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
{
    let listener = TcpListener::bind(address).await?;
    let acceptor = HttpAcceptor;

    if let Ok(address) = listener.local_addr() {
        // Call the listening callback with the address to which the TCP
        // listener is bound.
        on_ready(&address);
    } else {
        // Placeholder for tracing...
    }

    // Serve incoming connections from the TCP listener.
    serve(
        state,
        router,
        listener,
        acceptor,
        max_connections,
        shutdown_timeout,
    )
    .await
}

async fn listen_rustls<State>(
    state: Arc<State>,
    router: Arc<Router<State>>,
    key_path: Option<PathBuf>,
    cert_path: Option<PathBuf>,
    max_connections: usize,
    shutdown_timeout: Duration,
    address: impl ToSocketAddrs,
    on_ready: impl FnOnce(&SocketAddr),
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
{
    use tokio_rustls::TlsAcceptor;

    let (key_path, cert_path) = require_key_and_cert(key_path, cert_path)?;
    let listener = TcpListener::bind(address).await?;
    let tls_config = build_rustls_config(key_path, cert_path)?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    if let Ok(address) = listener.local_addr() {
        // Call the listening callback with the address to which the TCP
        // listener is bound.
        on_ready(&address);
    } else {
        // Placeholder for tracing...
    }

    // Serve incoming connections from the TCP listener.
    serve(
        state,
        router,
        listener,
        acceptor,
        max_connections,
        shutdown_timeout,
    )
    .await
}

fn build_rustls_config(
    key_path: PathBuf,
    cert_path: PathBuf,
) -> Result<rustls::ServerConfig, Error> {
    todo!()
    // rustls::ServerConfig::builder()
    //     .with_no_client_auth()
    //     .with_single_cert(cert_chain, key_der)
    //     .map_err(|error| error.into())
}

fn require_key_and_cert(
    key_path: Option<PathBuf>,
    cert_path: Option<PathBuf>,
) -> Result<(PathBuf, PathBuf), Error> {
    let key_path = key_path.ok_or_else(|| {
        let message = "key_path is required to use tls";
        Error::new(message.to_string())
    })?;

    let cert_path = cert_path.ok_or_else(|| {
        let message = "cert_path is required to use tls";
        Error::new(message.to_string())
    })?;

    Ok((key_path, cert_path))
}
