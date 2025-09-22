use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};

use super::accept::accept;
use crate::app::{App, AppService};
use crate::error::BoxError;

#[cfg(not(feature = "rustls"))]
use super::acceptor::HttpAcceptor;
#[cfg(feature = "rustls")]
use super::acceptor::{RustlsAcceptor, RustlsConfig};

/// Serve an app over HTTP.
///
pub struct Server<State> {
    app: App<State>,
    config: ServerConfig,

    #[cfg(feature = "rustls")]
    rustls_config: Option<RustlsConfig>,
}

#[derive(Debug)]
pub(super) struct ServerConfig {
    pub(super) accept_timeout: u64,
    pub(super) max_connections: usize,
    pub(super) max_request_size: usize,
    pub(super) shutdown_timeout: u64,
}

/// Creates a new server for the provided app.
///
pub fn serve<State>(app: App<State>) -> Server<State>
where
    State: Send + Sync + 'static,
{
    Server {
        app,
        config: Default::default(),
        #[cfg(feature = "rustls")]
        rustls_config: None,
    }
}

impl<State> Server<State>
where
    State: Send + Sync + 'static,
{
    /// The amount of time in seconds that the server will wait before the
    /// connection is reset if the server is at capacity.
    ///
    /// **Default:** `2s`
    ///
    pub fn accept_timeout(self, accept_timeout: u64) -> Self {
        Self {
            config: ServerConfig {
                accept_timeout,
                ..self.config
            },
            ..self
        }
    }

    /// Sets the maximum number of concurrent connections that the server can
    /// accept.
    ///
    /// **Default:** `1000`
    ///
    pub fn max_connections(self, max_connections: usize) -> Self {
        Self {
            config: ServerConfig {
                max_connections,
                ..self.config
            },
            ..self
        }
    }

    /// Set the maximum request body size in bytes.
    ///
    /// **Default:** `100 MB`
    ///
    pub fn max_request_size(self, max_request_size: usize) -> Self {
        Self {
            config: ServerConfig {
                max_request_size,
                ..self.config
            },
            ..self
        }
    }

    /// Set the amount of time in seconds that the server will wait for inflight
    /// connections to complete before shutting down.
    ///
    /// **Default:** `30s`
    ///
    pub fn shutdown_timeout(self, shutdown_timeout: u64) -> Self {
        Self {
            config: ServerConfig {
                shutdown_timeout,
                ..self.config
            },
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
    pub async fn listen<A>(self, address: A) -> Result<ExitCode, BoxError>
    where
        A: ToSocketAddrs,
    {
        let rustls_config = match self.rustls_config {
            Some(config) => Arc::new(config),
            None => panic!("rustls_config is required when the 'rustls' feature is enabled."),
        };

        let exit = accept(
            TcpListener::bind(address).await?,
            RustlsAcceptor::new(rustls_config),
            AppService::new(Arc::new(self.app), self.config.max_request_size),
            self.config,
        );

        Ok(exit.await)
    }

    #[cfg(not(feature = "rustls"))]
    pub async fn listen<A>(self, address: A) -> Result<ExitCode, BoxError>
    where
        A: ToSocketAddrs,
    {
        let exit = accept(
            TcpListener::bind(address).await?,
            HttpAcceptor::new(),
            AppService::new(Arc::new(self.app), self.config.max_request_size),
            self.config,
        );

        Ok(exit.await)
    }
}

#[cfg(feature = "rustls")]
impl<State: Send + Sync + 'static> Server<State> {
    /// Sets the TLS configuration for the server.
    ///
    pub fn rustls_config(self, rustls_config: RustlsConfig) -> Self {
        Self {
            rustls_config: Some(rustls_config),
            ..self
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            accept_timeout: 2,             // 2 seconds
            max_connections: 1000,         // 1000 concurrent connections
            max_request_size: 104_857_600, // 100 MB
            shutdown_timeout: 30,          // 30 seconds
        }
    }
}
