use std::process::ExitCode;
use tokio::net::{TcpListener, ToSocketAddrs};

#[cfg(feature = "rustls")]
use std::sync::Arc;

use super::accept::accept;
use crate::app::App;
use crate::error::DynError;

#[cfg(not(feature = "rustls"))]
use super::acceptor::HttpAcceptor;
#[cfg(feature = "rustls")]
use super::acceptor::{RustlsAcceptor, RustlsConfig};

/// The default value of the maximum number of concurrent connections.
///
const DEFAULT_MAX_CONNECTIONS: usize = 1024;

/// The default value of the maximum request body size in bytes (100MB).
///
const DEFAULT_MAX_BODY_SIZE: usize = 104_857_600;

/// The default value of the shutdown timeout in seconds.
///
const DEFAULT_SHUTDOWN_TIMEOUT: u64 = 30;

/// Serve an app over HTTP.
///
pub struct Server<T> {
    app: App<T>,
    max_body_size: Option<usize>,
    max_connections: Option<usize>,
    shutdown_timeout: Option<u64>,

    #[cfg(feature = "rustls")]
    rustls_config: Option<RustlsConfig>,
}

/// Creates a new server for the provided app.
///
pub fn start<T>(app: App<T>) -> Server<T> {
    Server {
        app,
        max_body_size: None,
        max_connections: None,
        shutdown_timeout: None,

        #[cfg(feature = "rustls")]
        rustls_config: None,
    }
}

impl<T: Send + Sync + 'static> Server<T> {
    /// Set the maximum request body size in bytes.
    ///
    /// Default: `100 MiB`
    ///
    pub fn max_body_size(self, limit: usize) -> Self {
        Self {
            max_body_size: Some(limit),
            ..self
        }
    }

    /// Sets the maximum number of concurrent connections that the server can
    /// accept.
    ///
    /// Default: `1024`
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

    /// Listens for incoming connections at the provided address.
    ///
    /// Returns a future that resolves with a result containing an [`ExitCode`]
    /// when shutdown is requested.
    ///
    /// # Errors
    ///
    /// - If the server fails to bind to the provided address.
    /// - If the `rustls` feature is enabled and `rustls_config` is missing.
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
    pub async fn listen<A: ToSocketAddrs>(self, address: A) -> Result<ExitCode, DynError> {
        let rustls_config = match self.rustls_config {
            Some(config) => Arc::new(config),
            None => panic!("rustls_config is required when the 'rustls' feature is enabled."),
        };

        let exit = accept(
            TcpListener::bind(address).await?,
            RustlsAcceptor::new(rustls_config),
            self.app,
            self.max_body_size.unwrap_or(DEFAULT_MAX_BODY_SIZE),
            self.max_connections.unwrap_or(DEFAULT_MAX_CONNECTIONS),
            self.shutdown_timeout.unwrap_or(DEFAULT_SHUTDOWN_TIMEOUT),
        );

        Ok(exit.await)
    }

    #[cfg(not(feature = "rustls"))]
    pub async fn listen<A: ToSocketAddrs>(self, address: A) -> Result<ExitCode, DynError> {
        let exit = accept(
            TcpListener::bind(address).await?,
            HttpAcceptor::new(),
            self.app,
            self.max_body_size.unwrap_or(DEFAULT_MAX_BODY_SIZE),
            self.max_connections.unwrap_or(DEFAULT_MAX_CONNECTIONS),
            self.shutdown_timeout.unwrap_or(DEFAULT_SHUTDOWN_TIMEOUT),
        );

        Ok(exit.await)
    }
}

#[cfg(feature = "rustls")]
impl<T: Send + Sync + 'static> Server<T> {
    /// Sets the TLS configuration for the server.
    ///
    pub fn rustls_config(self, rustls_config: RustlsConfig) -> Self {
        Self {
            rustls_config: Some(rustls_config),
            ..self
        }
    }
}
