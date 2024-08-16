use std::{net::SocketAddr, sync::Arc};

use crate::Error;

/// A function pointer to a callback that is called when an event occurs
/// outside the context of a request/response cycle.
pub type EventCallback<State> = fn(Event, &Arc<State>);

#[derive(Clone, Copy)]
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

pub(crate) struct EventListener<State> {
    callback: EventCallback<State>,
}

impl<State> EventListener<State> {
    pub fn new(callback: EventCallback<State>) -> Self {
        Self { callback }
    }

    pub fn call(&self, event: Event, state: &Arc<State>) {
        // TODO:
        //
        // Consider implementing an integrity check of some kind before calling
        // the callback. If the performance overhead is acceptable and doing so
        // mitigates the risk of an attacker exploiting `callback`, we'll avoid
        // having to clone Arc<EventListener> twice for every request.
        //
        // Alternatives:
        // - Use Arc<dyn Fn(Event, &Arc<State>) + Copy + Send + Sync + 'static>
        // - Use a channel to send events to a stream that exists for the
        //   lifetime of the application and spawn the event loop that accepts
        //   incoming TCP connections in a separate tokio task.
        //
        (self.callback)(event, state)
    }
}

impl<State> Copy for EventListener<State> {}

impl<State> Clone for EventListener<State> {
    #[allow(clippy::non_canonical_clone_impl)]
    fn clone(&self) -> Self {
        Self {
            callback: self.callback,
        }
    }
}
