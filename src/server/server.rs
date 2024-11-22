use std::future::Future;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, ToSocketAddrs};

#[cfg(feature = "rustls")]
use tokio_rustls::rustls;

use super::acceptor::{self, Acceptor};
use super::serve::serve;
use crate::app::App;
use crate::error::BoxError;
use crate::router::Router;

/// The default value of the maximum number of concurrent connections.
///
const DEFAULT_MAX_CONNECTIONS: usize = 256;

/// The default value of the maximum request body size in bytes (100MB).
///
const DEFAULT_MAX_REQUEST_SIZE: usize = 104_576_000;

/// The default value of the shutdown timeout in seconds.
///
const DEFAULT_SHUTDOWN_TIMEOUT: u64 = 30;

/// Serve an [`App`] over HTTP or HTTPS.
///
pub struct Server<State> {
    state: Arc<State>,
    router: Arc<Router<State>>,
    max_connections: Option<usize>,
    max_request_size: Option<usize>,
    shutdown_timeout: Option<u64>,

    #[cfg(feature = "rustls")]
    rustls_config: Option<rustls::ServerConfig>,
}

async fn listen<State, A>(
    acceptor: A,
    address: impl ToSocketAddrs,
    state: Arc<State>,
    router: Arc<Router<State>>,
    max_connections: Option<usize>,
    max_request_size: Option<usize>,
    shutdown_timeout: Option<u64>,
) -> Result<ExitCode, BoxError>
where
    State: Send + Sync + 'static,
    A: Acceptor + Send + Sync + 'static,
{
    let listener = TcpListener::bind(address).await?;
    let max_connections = max_connections.unwrap_or(DEFAULT_MAX_CONNECTIONS);
    let max_content_length = max_request_size.unwrap_or(DEFAULT_MAX_REQUEST_SIZE);
    let shutdown_timeout = shutdown_timeout.map_or_else(
        || Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT),
        Duration::from_secs,
    );

    if let Err(error) = listener.local_addr() {
        let _ = error;
        // Placeholder for tracing...
    }

    // Serve incoming connections until shutdown is requested.
    let server = serve(
        listener,
        acceptor,
        state,
        router,
        max_connections,
        max_content_length,
        shutdown_timeout,
    );

    server.await
}

impl<State> Server<State>
where
    State: Send + Sync + 'static,
{
    /// Creates a new server for the provided app.
    ///
    pub fn new(app: App<State>) -> Self {
        Self {
            state: app.state,
            router: Arc::new(app.router),
            #[cfg(feature = "rustls")]
            rustls_config: None,
            max_connections: None,
            max_request_size: None,
            shutdown_timeout: None,
        }
    }

    /// Starts the server and listens for incoming connections at the provided
    /// address. Returns a future that resolves with an [`ExitCode`] when the
    /// server is shutdown.
    ///
    /// # Errors
    ///
    /// If the server fails to bind to the provided address or is unable to
    /// finish serving all of the inflight connections within the specified
    /// [shutdown timeout](Self::shutdown_timeout)
    /// when a shutdown signal is received.
    ///
    /// # Panics
    ///
    /// If [`rustls_config`](Self::rustls_config)
    /// was not provided prior to calling this method.
    ///
    /// # Exit Codes
    ///
    /// An [`ExitCode::SUCCESS`] can be viewed as a confirmation that every
    /// request was served before exiting the accept loop.
    ///
    /// An [`ExitCode::FAILURE`] is an indicator that an unrecoverable error
    /// occured which requires that the server be restarted in order to function
    /// as intended.
    ///
    /// If you are running your Via application as a daemon with a process
    /// supervisor such as upstart or systemd, you can use the exit code to
    /// determine whether or not the process should restart.
    ///
    /// If you are running your Via application in a cluster behind a load
    /// balancer you can use the exit code to properly configure node replacement
    /// and / or decommissioning logic.
    ///
    /// When high availability is mission-critical, and you are scaling your Via
    /// application both horizontally and vertically using a combination of the
    /// aforementioned deployment strategies, we recommend configuring a temporal
    /// threshold for the number of restarts caused by an [`ExitCode::FAILURE`].
    /// If the threshold is exceeded the cluster should immutably replace the
    /// node and the process supervisor should not make further attempts to
    /// restart the process.
    ///
    /// This approach significantly reduces the impact of environmental entropy
    /// on your application's availability while preventing conflicts between the
    /// process supervisor of an individual node and the replacement and
    /// decommissioning logic of the cluster.
    ///
    #[cfg(feature = "rustls")]
    pub fn listen<A: ToSocketAddrs>(
        self,
        address: A,
    ) -> impl Future<Output = Result<ExitCode, BoxError>> {
        // Confirm that rustls_config exists before proceeding.

        let tls_config = match self.rustls_config {
            Some(config) => Arc::new(config),
            None => panic!("rustls_config is required to use the 'rustls' feature"),
        };

        // Create the RustlsAcceptor to serve connections over HTTPS.
        let acceptor = acceptor::rustls::RustlsAcceptor::new(tls_config);

        let state = self.state;
        let router = self.router;
        let max_connections = self.max_connections;
        let max_request_size = self.max_request_size;
        let shutdown_timeout = self.shutdown_timeout;

        listen(
            acceptor,
            address,
            state,
            router,
            max_connections,
            max_request_size,
            shutdown_timeout,
        )
    }

    /// Starts the server and listens for incoming connections at the provided
    /// address. Returns a future that resolves when the server is shutdown.
    ///
    /// # Errors
    ///
    /// If the server fails to bind to the provided address or is unable to
    /// finish serving all of the inflight connections within the specified
    /// [shutdown timeout](Self::shutdown_timeout)
    /// when a shutdown signal is received.
    ///
    /// # Exit Codes
    ///
    /// An [`ExitCode::SUCCESS`] can be viewed as a confirmation that every
    /// request was served before exiting the accept loop.
    ///
    /// An [`ExitCode::FAILURE`] is an indicator that an unrecoverable error
    /// occured which requires that the server be restarted in order to function
    /// as intended.
    ///
    /// If you are running your Via application as a daemon with a process
    /// supervisor such as upstart or systemd, you can use the exit code to
    /// determine whether or not the process should restart.
    ///
    /// If you are running your Via application in a cluster behind a load
    /// balancer you can use the exit code to properly configure node replacement
    /// and / or decommissioning logic.
    ///
    /// When high availability is mission-critical, and you are scaling your Via
    /// application both horizontally and vertically using a combination of the
    /// aforementioned deployment strategies, we recommend configuring a temporal
    /// threshold for the number of restarts caused by an [`ExitCode::FAILURE`].
    /// If the threshold is exceeded the cluster should immutably replace the
    /// node and the process supervisor should not make further attempts to
    /// restart the process.
    ///
    /// This approach significantly reduces the impact of environmental entropy
    /// on your application's availability while preventing conflicts between the
    /// process supervisor of an individual node and the replacement and
    /// decommissioning logic of the cluster.
    ///
    #[cfg(not(feature = "rustls"))]
    pub fn listen<A: ToSocketAddrs>(
        self,
        address: A,
    ) -> impl Future<Output = Result<ExitCode, BoxError>> {
        // Create a HttpAcceptor to serve connections over HTTP.
        let acceptor = acceptor::http::HttpAcceptor;

        let state = self.state;
        let router = self.router;
        let max_connections = self.max_connections;
        let max_request_size = self.max_request_size;
        let shutdown_timeout = self.shutdown_timeout;

        listen(
            acceptor,
            address,
            state,
            router,
            max_connections,
            max_request_size,
            shutdown_timeout,
        )
    }

    /// Set the amount of time in seconds that the server will wait for inflight
    /// connections to complete before shutting down. The default value is 30
    /// seconds.
    ///
    pub fn shutdown_timeout(self, timeout: u64) -> Self {
        Self {
            shutdown_timeout: Some(timeout),
            ..self
        }
    }

    pub fn max_request_size(self, limit: usize) -> Self {
        Self {
            max_request_size: Some(limit),
            ..self
        }
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
    pub fn max_connections(self, limit: usize) -> Self {
        Self {
            max_connections: Some(limit),
            ..self
        }
    }

    /// Sets the TLS configuration for the server.
    ///
    #[cfg(feature = "rustls")]
    pub fn rustls_config(self, server_config: rustls::ServerConfig) -> Self {
        Self {
            rustls_config: Some(server_config),
            ..self
        }
    }
}
