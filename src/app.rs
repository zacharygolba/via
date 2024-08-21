use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::router::{Endpoint, Router};
use crate::server::serve;
use crate::{Error, Middleware};

const DEFAULT_MAX_CONNECTIONS: usize = 256;

pub struct App<State> {
    max_connections: usize,
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
        T: Middleware<State>,
    {
        self.at("/").include(middleware);
        self
    }

    pub fn set_max_connections(&mut self, n: usize) -> &mut Self {
        self.max_connections = n;
        self
    }

    pub async fn listen<F, T>(mut self, address: T, listening: F) -> Result<(), Error>
    where
        F: FnOnce(&SocketAddr),
        T: ToSocketAddrs,
    {
        // Shrink the router to fit the number of routes that have been added.
        self.router.shrink_to_fit();

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
        serve(state, router, listener, self.max_connections).await
    }
}
