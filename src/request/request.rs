use http::{HeaderMap, Method, Uri, Version};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use crate::event::EventListener;

use super::{
    path_param::PathParams,
    query_param::{QueryParamValues, QueryParams},
    query_parser::parse_query_params,
    Body, PathParam,
};

pub struct Request<State> {
    inner: http::Request<Body>,
    app_state: Arc<State>,
    path_params: PathParams,
    query_params: Option<QueryParams>,
    event_listener: EventListener,
}

impl<State> Request<State> {
    pub(crate) fn new(
        inner: http::Request<Body>,
        app_state: Arc<State>,
        path_params: PathParams,
        event_listener: EventListener,
    ) -> Self {
        Self {
            inner,
            app_state,
            path_params,
            event_listener,
            query_params: None,
        }
    }

    /// Returns a reference to the body associated with the request.
    pub fn body(&self) -> &Body {
        self.inner.body()
    }

    /// Returns a mutable reference to the body associated with the request.
    pub fn body_mut(&mut self) -> &mut Body {
        self.inner.body_mut()
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// Returns a reference to the HTTP method associated with the request.
    pub fn method(&self) -> &Method {
        self.inner.method()
    }

    pub fn param<'a>(&self, name: &'a str) -> PathParam<'_, 'a> {
        let path = self.inner.uri().path();

        for (param, range) in &self.path_params {
            if name == *param {
                return PathParam::new(name, path, Some(range));
            }
        }

        PathParam::new(name, path, None)
    }

    pub fn query<'a>(&mut self, name: &'a str) -> QueryParamValues<'_, 'a> {
        let mut values = Vec::new();

        let query = self.inner.uri().query().unwrap_or("");
        let params = self
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
        &self.app_state
    }

    /// Returns a reference to the uri associated with the request.
    pub fn uri(&self) -> &Uri {
        self.inner.uri()
    }

    /// Returns the HTTP version associated with the request.
    pub fn version(&self) -> Version {
        self.inner.version()
    }
}

impl<State> Request<State> {
    pub(crate) fn event_listener(&self) -> &EventListener {
        &self.event_listener
    }
}

impl<State> Debug for Request<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", self.method())
            .field("uri", self.uri())
            .field("params", &self.path_params)
            .field("query", &self.query_params)
            .field("version", &self.version())
            .field("headers", self.headers())
            .field("body", self.body())
            .finish()
    }
}
