use http::{HeaderMap, Method, Uri, Version};
use hyper::body::Incoming;
use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use super::{parse_query_params, Body, PathParam, QueryParamValues};
use crate::event::EventListener;

pub struct Request<State = ()> {
    inner: Box<RequestInner<State>>,
}

struct RequestInner<State> {
    request: http::Request<Body>,
    app_state: Arc<State>,
    path_params: Vec<(&'static str, (usize, usize))>,
    query_params: Option<Vec<(String, (usize, usize))>>,
    event_listener: EventListener,
}

impl<State> Request<State> {
    /// Returns a reference to the body associated with the request.
    pub fn body(&self) -> &Body {
        self.inner.request.body()
    }

    /// Returns a mutable reference to the body associated with the request.
    pub fn body_mut(&mut self) -> &mut Body {
        self.inner.request.body_mut()
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    pub fn headers(&self) -> &HeaderMap {
        self.inner.request.headers()
    }

    /// Returns a reference to the HTTP method associated with the request.
    pub fn method(&self) -> &Method {
        self.inner.request.method()
    }

    pub fn param<'a>(&self, name: &'a str) -> PathParam<'_, 'a> {
        let path = self.inner.request.uri().path();

        for (param, range) in &self.inner.path_params {
            if name == *param {
                return PathParam::new(name, path, Some(range));
            }
        }

        PathParam::new(name, path, None)
    }

    pub fn query<'a>(&mut self, name: &'a str) -> QueryParamValues<'_, 'a> {
        let mut values = Vec::new();

        let query = self.inner.request.uri().query().unwrap_or("");
        let params = self
            .inner
            .query_params
            .get_or_insert_with(|| parse_query_params(query));

        for (param, range) in params.iter() {
            if name == param {
                values.push(range);
            }
        }

        QueryParamValues::new(name, query, values)
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the `via::app` function.
    pub fn state(&self) -> &Arc<State> {
        &self.inner.app_state
    }

    /// Returns a reference to the uri associated with the request.
    pub fn uri(&self) -> &Uri {
        self.inner.request.uri()
    }

    /// Returns the HTTP version associated with the request.
    pub fn version(&self) -> Version {
        self.inner.request.version()
    }
}

impl<State> Request<State> {
    pub(crate) fn new(
        request: http::Request<Incoming>,
        app_state: Arc<State>,
        event_listener: EventListener,
    ) -> Self {
        Self {
            // Box the request and map the request body to `request::Body` to
            // move both the request and body independently to the heap. Doing
            // so keeps the size of the request small and allows the body to be
            // easily moved out of the request when it is read.
            inner: Box::new(RequestInner {
                app_state,
                event_listener,
                request: request.map(Body::new),
                path_params: Vec::with_capacity(10),
                query_params: None,
            }),
        }
    }

    pub(crate) fn event_listener(&self) -> &EventListener {
        &self.inner.event_listener
    }

    pub(crate) fn params_mut(&mut self) -> &mut Vec<(&'static str, (usize, usize))> {
        &mut self.inner.path_params
    }
}

impl<State> Debug for Request<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", self.method())
            .field("uri", self.uri())
            .field("params", &self.inner.path_params)
            .field("query", &self.inner.query_params)
            .field("version", &self.version())
            .field("headers", self.headers())
            .field("body", self.body())
            .finish()
    }
}
