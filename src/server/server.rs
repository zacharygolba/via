use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::ToSocketAddrs;

use crate::app::{AppService, Via};
use crate::error::Error;

#[cfg(any(feature = "native-tls", feature = "rustls"))]
use super::tls;

/// Serve an app over HTTP.
///
pub struct Server<App> {
    app: Via<App>,
    config: ServerConfig,
}

#[derive(Debug)]
pub(super) struct ServerConfig {
    pub(super) max_connections: usize,
    pub(super) max_request_size: usize,
    pub(super) shutdown_timeout: Duration,
}

impl<App> Server<App>
where
    App: Send + Sync + 'static,
{
    /// Creates a new server for the provided app.
    ///
    pub fn new(app: Via<App>) -> Self {
        Self {
            app,
            config: Default::default(),
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
    pub fn shutdown_timeout(self, shutdown_timeout: Duration) -> Self {
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
    pub fn listen<A>(self, address: A) -> impl Future<Output = Result<ExitCode, Error>>
    where
        A: ToSocketAddrs,
    {
        use tokio::net::TcpListener;

        let Self { app, config } = self;
        let service = AppService::new(Arc::new(app), config.max_request_size);

        async {
            let exit = super::accept(
                config,
                TcpListener::bind(address).await?,
                Box::new(|stream| async { Ok(stream) }),
                service,
            );

            Ok(exit.await)
        }
    }

    #[cfg(feature = "native-tls")]
    pub fn listen_native_tls<A>(
        self,
        address: A,
        tls_config: native_tls::Identity,
    ) -> impl Future<Output = Result<ExitCode, Error>>
    where
        A: ToSocketAddrs,
    {
        let Self { app, config, .. } = self;
        let service = AppService::new(Arc::new(app), config.max_request_size);

        tls::listen_native_tls(config, address, tls_config, service)
    }

    #[cfg(feature = "rustls")]
    pub fn listen_rustls<A>(
        self,
        address: A,
        tls_config: rustls::ServerConfig,
    ) -> impl Future<Output = Result<ExitCode, Error>>
    where
        A: ToSocketAddrs,
    {
        let Self { app, config, .. } = self;
        let service = AppService::new(Arc::new(app), config.max_request_size);

        tls::listen_rustls(config, address, tls_config, service)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            max_request_size: 104_857_600, // 100 MB
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}
