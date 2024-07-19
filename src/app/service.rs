use hyper::{body::Incoming, service::Service};
use std::{convert::Infallible, sync::Arc};

use crate::{
    event::{Event, EventListener},
    middleware::BoxFuture,
    router::Router,
    App, Request, Result,
};

pub struct AppService<State> {
    pub event_listener: EventListener,
    pub router: Arc<Router<State>>,
    pub state: Arc<State>,
}

impl<State> AppService<State> {
    pub fn new<T>(app: App<State>, event_listener: T) -> Self
    where
        T: Fn(Event) + Send + Sync + 'static,
    {
        Self {
            event_listener: EventListener::new(event_listener),
            router: Arc::new(app.router),
            state: Arc::new(app.state),
        }
    }
}

impl<State> Clone for AppService<State> {
    fn clone(&self) -> Self {
        Self {
            event_listener: self.event_listener.clone(),
            router: Arc::clone(&self.router),
            state: Arc::clone(&self.state),
        }
    }
}

impl<State> Service<http::Request<Incoming>> for AppService<State>
where
    State: Send + Sync + 'static,
{
    type Error = Infallible;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;
    type Response = http::Response<crate::response::Body>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut request = {
            let event_listener = self.event_listener.clone();
            let state = Arc::clone(&self.state);

            Request::new(request, state, event_listener)
        };

        let event_listener = self.event_listener.clone();
        let next = self.router.visit(&mut request);

        Box::pin(async move {
            let response = next.call(request).await.unwrap_or_else(|error| {
                error.into_infallible_response(|error| {
                    // If the error was not able to be converted into a response,
                    // with the configured error format (i.e json), fallback to a
                    // plain text response and propagate the reason why the error
                    // could not be converted to the event listener so it can be
                    // handled at the application level.
                    event_listener.call(Event::UncaughtError(error));
                })
            });

            Ok(response.into_inner())
        })
    }
}
