use http::{header, HeaderMap, Method, Uri, Version};
use hyper::body::Incoming;
use std::fmt::{self, Debug};
use std::sync::Arc;

use super::{parse_query_params, PathParam, PathParams, QueryParamValues};
use crate::body::RequestBody;
use crate::{Error, Result};

pub struct Request<State = ()> {
    inner: Box<RequestInner<State>>,
}

struct RequestInner<State> {
    request: http::Request<Option<RequestBody>>,
    app_state: Arc<State>,
    path_params: PathParams,
    query_params: Option<Vec<(String, (usize, usize))>>,
}

impl<State> Request<State> {
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
            if name == &**param {
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

    pub fn take_body(&mut self) -> Result<RequestBody> {
        match self.inner.request.body_mut().take() {
            Some(body) => Ok(body),
            None => Err(Error::new("body has already been read".to_owned())),
        }
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
    pub(crate) fn new(request: http::Request<Incoming>, app_state: Arc<State>) -> Self {
        let content_len =
            request
                .headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|value| match value.to_str() {
                    Ok(value) => value.parse::<usize>().ok(),
                    Err(_) => None,
                });

        Self {
            // Box the request and map the request body to `request::Body` to
            // move both the request and body independently to the heap. Doing
            // so keeps the size of the request small and allows the body to be
            // easily moved out of the request when it is read.
            inner: Box::new(RequestInner {
                app_state,
                request: request.map(|body| match content_len {
                    Some(len) => Some(RequestBody::with_len(body, len)),
                    None => Some(RequestBody::new(body)),
                }),
                path_params: Vec::with_capacity(10),
                query_params: None,
            }),
        }
    }

    pub(crate) fn params_mut(&mut self) -> &mut PathParams {
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
            .field("body", self.inner.request.body())
            .finish()
    }
}
