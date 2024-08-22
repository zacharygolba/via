use http::header::CONTENT_LENGTH;
use http::request::Parts;
use http::{HeaderMap, Method, Uri, Version};
use hyper::body::Incoming;
use std::fmt::{self, Debug};
use std::sync::Arc;

use super::params::{Param, Params, QueryParamValues};
use crate::body::RequestBody;

pub struct Request<State = ()> {
    /// The component parts of the underlying HTTP request.
    parts: Box<Parts>,

    /// The shared application state associated with the request.
    state: Arc<State>,

    /// The request's body.
    body: RequestBody,

    /// The request's path and query parameters.
    params: Params,
}

fn get_content_len(headers: &HeaderMap) -> Option<usize> {
    match headers.get(CONTENT_LENGTH)?.to_str() {
        Ok(value) => value.parse::<usize>().ok(),
        Err(_) => None,
    }
}

impl<State> Request<State> {
    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    pub fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    /// Returns a reference to the HTTP method associated with the request.
    pub fn method(&self) -> &Method {
        &self.parts.method
    }

    /// Returns a convenient wrapper around an optional reference to the path
    /// parameter in the request's uri with the provided `name`.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{Error, Next, Request};
    ///
    /// async fn hello(request: Request, _: Next) -> Result<String, Error> {
    ///     let required: Result<&str, Error> = request.param("name").required();
    ///     let _optional: Option<&str> = request.param("name").ok();
    ///
    ///     Ok(format!("Hello, {}!", required?))
    /// }
    /// ```
    pub fn param<'a>(&self, name: &'a str) -> Param<'_, 'a> {
        // Get the path of the request's uri.
        let path = self.parts.uri.path();
        // Get an `Option<(usize, usize)>` that represents the start and end
        // offset of the path parameter with the provided `name` in the request's
        // uri.
        let at = self.params.get_path_param(name);

        Param::new(at, name, path)
    }

    /// Returns a convenient wrapper around an optional references to the query
    /// parameters in the request's uri with the provided `name`.
    pub fn query<'a>(&mut self, name: &'a str) -> QueryParamValues<'_, 'a> {
        self.parts.uri.query().map_or_else(
            || QueryParamValues::empty(name),
            |query| self.params.get_query_params(query, name),
        )
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the [`via::app`](crate::app::app)
    /// function.
    pub fn state(&self) -> &Arc<State> {
        &self.state
    }

    /// Returns a reference to the uri associated with the request.
    pub fn uri(&self) -> &Uri {
        &self.parts.uri
    }

    /// Returns the HTTP version associated with the request.
    pub fn version(&self) -> Version {
        self.parts.version
    }

    /// Consumes the request and returns the body.
    pub fn into_body(self) -> RequestBody {
        self.body
    }

    /// Consumes the request and returns a tuple containing the boxed component
    /// parts of the request, the body, and an `Arc` to the shared application
    /// state.
    pub fn into_parts(self) -> (Parts, RequestBody, Arc<State>) {
        (*self.parts, self.body, self.state)
    }
}

impl<State> Request<State> {
    pub(crate) fn new(request: http::Request<Incoming>, params: Params, state: Arc<State>) -> Self {
        // Destructure the `http::Request` into its component parts.
        let (parts, body) = request.into_parts();
        // Box the component parts of the request to keep the size of the request
        // type small. This is important because the request type is passed by
        // value to middleware and responder functions.
        let parts = Box::new(parts);
        // Check if the request has a `Content-Length` header. If it does, wrap
        // the request body in a `RequestBody` with a known length. Otherwise,
        // wrap the request body in a `RequestBody` with an unknown length.
        let body = match get_content_len(&parts.headers) {
            Some(len) => RequestBody::with_len(body, len),
            None => RequestBody::new(body),
        };

        Self {
            params,
            parts,
            state,
            body,
        }
    }
}

impl<State> Debug for Request<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", self.method())
            .field("uri", self.uri())
            .field("params", &self.params)
            .field("version", &self.version())
            .field("headers", self.headers())
            .field("body", &self.body)
            .finish()
    }
}
