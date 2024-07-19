use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{
    event::Event,
    router::{Endpoint, Router},
    Error, Middleware, Result,
};

use super::service::AppService;

pub struct App<State> {
    pub(super) router: Router<State>,
    pub(super) state: State,
}

pub fn app<State>(state: State) -> App<State>
where
    State: Send + Sync + 'static,
{
    App {
        state,
        router: Router::new(),
    }
}

async fn serve<State>(tcp_listener: TcpListener, app_service: AppService<State>) -> Result<()>
where
    State: Send + Sync + 'static,
{
    loop {
        // Accept a new connection from the TCP listener.
        let (stream, _) = tcp_listener.accept().await?;
        // Pass the returned stream to the `TokioIo` wrapper to convert
        // the stream into a tokio-compatible I/O stream.
        let io = TokioIo::new(stream);

        // Clone the `AppService` so it can be moved into the tokio task.
        // Each field of service is either wrapped in an `Arc` or contains
        // an `Arc`. Therefore, cloning the service is a relatively cheap
        // operation.
        let app_service = app_service.clone();

        // Spawn a tokio task to serve multiple connections concurrently.
        tokio::spawn(async move {
            // Create a new connection for the configured HTTP version. For
            // now we only support HTTP/1.1. This will be expanded to
            // support HTTP/2 in the future.
            let connection = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, &app_service);

            if let Err(error) = connection.await {
                let error = Error::from(error);
                let event = Event::ConnectionError(&error);

                // A connection error occured while serving the connection.
                // Propagate the error to the event listener so it can be
                // handled at the application level.
                app_service.event_listener.call(event);
            }
        });
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

    pub async fn listen<T, F>(self, address: T, event_listener: F) -> Result<()>
    where
        T: ToSocketAddrs,
        F: Fn(Event) + Send + Sync + 'static,
    {
        let tcp_listener = TcpListener::bind(address).await?;
        let app_service = AppService::new(self, event_listener);

        {
            // Notify the event listener that the server is ready to accept
            // incoming connections at the address to which the TCP listener
            // is bound.

            let address = tcp_listener.local_addr()?;
            let event = Event::ServerReady(&address);

            app_service.event_listener.call(event);
        }

        serve(tcp_listener, app_service).await
    }
}
