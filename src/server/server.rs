use std::sync::Arc;
use std::time::Duration;
use std::{future::Future, net::SocketAddr};
use tokio::net::{TcpListener, ToSocketAddrs};
#[cfg(feature = "rustls")]
use tokio_rustls::rustls;
#[cfg(feature = "rustls")]
use tokio_rustls::TlsAcceptor;

#[cfg(not(feature = "rustls"))]
use super::acceptor::HttpAcceptor;
use super::serve;
use crate::{App, Error, Router};

/// The default value of the maximum number of concurrent connections.
const DEFAULT_MAX_CONNECTIONS: usize = 256;

/// The default value of the shutdown timeout in seconds.
const DEFAULT_SHUTDOWN_TIMEOUT: u64 = 30;

pub struct Server<State> {
    state: Arc<State>,
    router: Arc<Router<State>>,
    #[cfg(feature = "rustls")]
    rustls_config: Option<rustls::ServerConfig>,
    max_connections: Option<usize>,
    shutdown_timeout: Option<u64>,
}

impl<State> Server<State>
where
    State: Send + Sync + 'static,
{
    pub fn new(app: App<State>) -> Self {
        let (state, router) = app.into_parts();

        Self {
            state,
            router: Arc::new(router),
            #[cfg(feature = "rustls")]
            rustls_config: None,
            max_connections: None,
            shutdown_timeout: None,
        }
    }

    #[cfg(feature = "rustls")]
    pub fn rustls_config(mut self, server_config: rustls::ServerConfig) -> Self {
        self.rustls_config = Some(server_config);
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

    pub fn listen<A, R>(self, address: A, on_ready: R) -> impl Future<Output = Result<(), Error>>
    where
        A: ToSocketAddrs,
        R: FnOnce(&SocketAddr),
    {
        let listener_future = async {
            let listener = TcpListener::bind(address).await?;

            if let Ok(address) = listener.local_addr() {
                // Call the listening callback with the address to which the TCP
                // listener is bound.
                on_ready(&address);
            } else {
                // Placeholder for tracing...
            }

            Ok(listener)
        };

        let state = self.state;
        let router = self.router;
        let max_connections = self.max_connections.unwrap_or(DEFAULT_MAX_CONNECTIONS);
        let shutdown_timeout = self.shutdown_timeout.map_or_else(
            || Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT),
            Duration::from_secs,
        );

        #[cfg(feature = "rustls")]
        let future = serve_rustls(
            listener_future,
            state,
            router,
            max_connections,
            shutdown_timeout,
        );

        #[cfg(not(feature = "rustls"))]
        let future = serve_http(
            listener_future,
            state,
            router,
            max_connections,
            shutdown_timeout,
        );

        future
    }
}

#[cfg(feature = "rustls")]
pub async fn serve_rustls<State, F>(
    listener_future: F,
    state: Arc<State>,
    router: Arc<Router<State>>,
    rustls_config: Option<rustls::ServerConfig>,
    max_connections: usize,
    shutdown_timeout: Duration,
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
    F: Future<Output = Result<TcpListener, Error>>,
{
    let listener = listener_future.await?;

    let acceptor = {
        let rustls_config = rustls_config.ok_or_else(|| {
            let message = "rustls_config is required to use the 'rustls' feature";
            Error::new(message.to_string())
        })?;

        TlsAcceptor::from(Arc::new(rustls_config))
    };

    let shutdown = serve(
        state,
        router,
        listener,
        acceptor,
        max_connections,
        shutdown_timeout,
    );

    shutdown.await
}

#[cfg(not(feature = "rustls"))]
pub async fn serve_http<State, F>(
    listener_future: F,
    state: Arc<State>,
    router: Arc<Router<State>>,
    max_connections: usize,
    shutdown_timeout: Duration,
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
    F: Future<Output = Result<TcpListener, Error>>,
{
    let listener = listener_future.await?;
    let acceptor = HttpAcceptor;
    let shutdown = serve(
        state,
        router,
        listener,
        acceptor,
        max_connections,
        shutdown_timeout,
    );

    shutdown.await
}
