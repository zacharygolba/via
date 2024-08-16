use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::task;

use crate::body::Pollable;
use crate::router::{Endpoint, Router};
use crate::{Error, Middleware, Request};

/// The request type used by our `hyper` service. This is the type that we will
/// use to create a `via::Request` that will be passed to the middleware stack.
type HttpRequest = http::Request<Incoming>;

/// The response type used by our `hyper` service. This is the type that we will
/// unwrap from the `via::Response` returned from the middleware stack.
type HttpResponse = http::Response<Pollable>;

pub struct App<State> {
    router: Router<State>,
    state: Arc<State>,
}

/// Constructs a new `App` with the provided `state`.
pub fn app<State>(state: State) -> App<State>
where
    State: Send + Sync + 'static,
{
    App {
        router: Router::new(),
        state: Arc::new(state),
    }
}

async fn serve<State>(
    state: Arc<State>,
    router: Arc<Router<State>>,
    tcp_listener: TcpListener,
) -> Result<(), Error>
where
    State: Send + Sync + 'static,
{
    loop {
        // Accept a new connection from the TCP listener.
        let (stream, _) = tcp_listener.accept().await?;
        // Pass the returned stream to the `TokioIo` wrapper to convert
        // the stream into a tokio-compatible I/O stream.
        let io = TokioIo::new(stream);

        // Clone the `state` so it can be moved into the task.
        let state = Arc::clone(&state);
        // Clone the `router` so it can be moved into the task.
        let router = Arc::clone(&router);

        // Spawn a tokio task to serve multiple connections concurrently.
        task::spawn(async move {
            // Create a hyper service from the `serve_request` function.
            let service = service_fn(|request| serve_request(&state, &router, request));

            // Create a new connection for the configured HTTP version. For
            // now we only support HTTP/1.1. This will be expanded to
            // support HTTP/2 in the future.
            let connection = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service);

            if let Err(error) = connection.await {
                //
                // TODO:
                //
                // Replace eprintln with pretty_env_logger or something similar.
                // We should also determine if this is how we want to handle
                // connection errors long-term.
                //
                if cfg!(debug_assertions) {
                    eprintln!("Error: {}", error);
                }
            }
        });
    }
}

/// Serves an incoming request by routing it to the corresponding middleware
/// stack and returns a response.
async fn serve_request<State>(
    state: &Arc<State>,
    router: &Arc<Router<State>>,
    request: HttpRequest,
) -> Result<HttpResponse, Error>
where
    State: Send + Sync + 'static,
{
    // Wrap the incoming request in with `via::Request`.
    //
    // Note:
    //
    // In the future, we may want to pass a reference to `state` to avoid having
    // to incremet the ref count of the `Arc` unnecessarily.
    //
    let mut request = Request::new(request, Arc::clone(&state));

    // Route `request` to the corresponding middleware stack.
    let next = router.visit(&mut request);

    // Call the middleware stack and return a response.
    match next.call(request).await {
        Ok(response) => Ok(response.into_inner()),
        Err(error) => Ok(error.into_response().into_inner()),
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

    pub async fn listen<F, T>(self, address: T, listening: F) -> Result<(), Error>
    where
        F: FnOnce(&SocketAddr),
        T: ToSocketAddrs,
    {
        let state = self.state;
        let router = Arc::new(self.router);
        let tcp_listener = TcpListener::bind(address).await?;

        if let Ok(address) = tcp_listener.local_addr() {
            // Notify the event listener that the server is ready to accept
            // incoming connections at the address to which the TCP listener
            // is bound.
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
        serve(state, router, tcp_listener).await
    }
}
