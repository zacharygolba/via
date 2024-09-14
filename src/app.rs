use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::router::{Endpoint, Router};
use crate::server::serve;
use crate::{Error, Middleware};

/// The default value of the maximum number of concurrent connections.
const DEFAULT_MAX_CONNECTIONS: usize = 256;

/// The default value of the shutdown timeout in seconds.
const DEFAULT_SHUTDOWN_TIMEOUT: u32 = 30;

pub struct App<State> {
    max_connections: usize,
    shutdown_timeout: u32,
    router: Router<State>,
    state: Arc<State>,
}

/// Constructs a new `App` with the provided `state`.
pub fn app<State>(state: State) -> App<State>
where
    State: Send + Sync + 'static,
{
    App {
        max_connections: DEFAULT_MAX_CONNECTIONS,
        shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
        router: Router::new(),
        state: Arc::new(state),
    }
}

impl<State> App<State>
where
    State: Send + Sync + 'static,
{
    pub fn at(&mut self, pattern: &'static str) -> Endpoint<State> {
        self.router.at(pattern)
    }

    pub fn include<T>(&mut self, middleware: T) -> &mut Self
    where
        T: Middleware<State> + 'static,
    {
        self.at("/").include(middleware);
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
        self.max_connections = n;
        self
    }

    /// Set the amount of time in seconds that the server will wait for inflight
    /// connections to complete before shutting down. The default value is 30
    /// seconds.
    pub fn shutdown_timeout(mut self, timeout: u32) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    pub async fn listen<F, T>(self, address: T, listening: F) -> Result<(), Error>
    where
        F: FnOnce(&SocketAddr),
        T: ToSocketAddrs,
    {
        let state = self.state;
        let router = Arc::new(self.router);
        let listener = TcpListener::bind(address).await?;

        if let Ok(address) = listener.local_addr() {
            // Call the listening callback with the address to which the TCP
            // listener is bound.
            listening(&address);
        } else {
            // TODO:
            //
            // Handle the case where the TCP listener is unable to retrieve
            // the local address and determine how we should handle it. My
            // instinct says that we should panic with an opaque yet descriptive
            // error message.
        }

        // Serve incoming connections from the TCP listener.
        serve(
            state,
            router,
            listener,
            self.max_connections,
            Duration::from_secs(self.shutdown_timeout.into()),
        )
        .await
    }
}
