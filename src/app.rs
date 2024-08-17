use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::router::{Endpoint, Router};
use crate::server::serve;
use crate::{Error, Middleware};

pub struct App<State> {
    max_connections: Option<usize>,
    router: Router<State>,
    state: Arc<State>,
}

/// Constructs a new `App` with the provided `state`.
pub fn app<State>(state: State) -> App<State>
where
    State: Send + Sync + 'static,
{
    App {
        max_connections: None,
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
        self.max_connections = Some(n);
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
        serve(state, router, listener, self.max_connections).await
    }
}
