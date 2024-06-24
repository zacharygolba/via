use std::{net::SocketAddr, sync::Arc};

use crate::Error;

#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum Event<'a> {
    /// An error occurred with a connection to the application server. This
    /// event only occurs if an application fails to respond to a request.
    /// For example, if the request times out before the application has
    /// a chance to respond.
    ConnectionError(&'a Error),

    /// The server is ready to accept incoming connections at the given address.
    ServerReady(&'a SocketAddr),

    /// A recoverable error occured while processing a request. This event
    /// is dispatched when an error occurs but the application is still able
    /// to respond to the request. For example, if an `ErrorBoundary` is called
    /// because of an error that occured upstream in the middleware stack but the
    /// error is unable to be serialized into the desired response format.
    UncaughtError(&'a Error),
}

pub(crate) struct EventListener {
    f: Arc<dyn Fn(Event) + Send + Sync + 'static>,
}

impl EventListener {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        Self { f: Arc::new(f) }
    }

    pub fn call(&self, event: Event) {
        (self.f)(event)
    }
}

impl Clone for EventListener {
    fn clone(&self) -> Self {
        Self {
            f: Arc::clone(&self.f),
        }
    }
}
