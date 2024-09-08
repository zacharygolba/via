use bytes::Bytes;
use http::request::Parts;
use http::{HeaderMap, Method, Uri, Version};
use hyper::body::{Body, Incoming};
use std::fmt::{self, Debug};
use std::sync::Arc;

use super::params::{Param, PathParams, QueryParam};
use crate::body::{AnyBody, Boxed};
use crate::Error;

pub struct Request<State = ()> {
    inner: Box<RequestData<State>>,
}

struct RequestData<State> {
    /// The component parts of the underlying HTTP request.
    parts: Parts,

    /// The shared application state associated with the request.
    state: Arc<State>,

    /// The request's path and query parameters.
    params: PathParams,

    /// The request's body.
    body: AnyBody<Incoming>,
}

impl<State> Request<State> {
    /// Consumes the request returning a new request with body mapped to the
    /// return type of the passed in function.
    pub fn map<F, B>(self, f: F) -> Self
    where
        F: FnOnce(AnyBody<Incoming>) -> B,
        B: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        let inner = *self.inner;
        let output = f(inner.body);

        Self {
            inner: Box::new(RequestData {
                body: AnyBody::Boxed(Boxed::new(Box::pin(output))),
                ..inner
            }),
        }
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    pub fn headers(&self) -> &HeaderMap {
        &self.inner.parts.headers
    }

    /// Returns a reference to the HTTP method associated with the request.
    pub fn method(&self) -> &Method {
        &self.inner.parts.method
    }

    /// Returns a convenient wrapper around an optional reference to the path
    /// parameter in the request's uri with the provided `name`.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{Error, Next, Request};
    /// use std::borrow::Cow;
    ///
    /// async fn hello(request: Request, _: Next) -> Result<String, Error> {
    ///     let required: Result<Cow<str>, Error> = request.param("name").required();
    ///     let _optional: Option<Cow<str>> = request.param("name").ok();
    ///
    ///     Ok(format!("Hello, {}!", required?))
    /// }
    /// ```
    pub fn param<'a>(&self, name: &'a str) -> Param<'_, 'a> {
        // Get the path of the request's uri.
        let path = self.inner.parts.uri.path();
        // Get an `Option<(usize, usize)>` that represents the start and end
        // offset of the path parameter with the provided `name` in the request's
        // uri.
        let at = self.inner.params.get(name);

        Param::new(Some(at), name, path)
    }

    /// Returns a convenient wrapper around an optional references to the query
    /// parameters in the request's uri with the provided `name`.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{Next, Request, Result};
    ///
    /// async fn hello(request: Request, _: Next) -> Result<String> {
    ///     // Attempt to parse the first query parameter named `n` to a `usize`
    ///     // no greater than 1000. If the query parameter doesn't exist or
    ///     // can't be converted to a `usize`, default to 1.
    ///     let n = request.query("n").first().parse().unwrap_or(1).min(1000);
    ///
    ///     // Get a reference to the path parameter `name` from the request uri.
    ///     let name = request.param("name").required()?;
    ///
    ///     // Create a greeting message with the provided `name`.
    ///     let message = format!("Hello, {}!\n", name);
    ///
    ///     // Send a response with our greeting message, repeated `n` times.
    ///     Ok(message.repeat(n))
    /// }
    /// ```
    pub fn query<'a>(&self, name: &'a str) -> QueryParam<'_, 'a> {
        let query = self.inner.parts.uri.query().unwrap_or("");

        QueryParam::new(name, query)
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the [`via::app`](crate::app::app)
    /// function.
    pub fn state(&self) -> &Arc<State> {
        &self.inner.state
    }

    /// Returns a reference to the uri associated with the request.
    pub fn uri(&self) -> &Uri {
        &self.inner.parts.uri
    }

    /// Returns the HTTP version associated with the request.
    pub fn version(&self) -> Version {
        self.inner.parts.version
    }

    /// Consumes the request and returns the body.
    pub fn into_body(self) -> AnyBody<Incoming> {
        self.inner.body
    }

    /// Unwraps `self` into the Request type from the `http` crate.
    pub fn into_inner(self) -> http::Request<AnyBody<Incoming>> {
        let (parts, body) = self.into_parts();
        http::Request::from_parts(parts, body)
    }

    /// Consumes the request and returns a tuple containing the component
    /// parts of the request and the request body.
    pub fn into_parts(self) -> (Parts, AnyBody<Incoming>) {
        (self.inner.parts, self.inner.body)
    }
}

impl<State> Request<State> {
    pub(crate) fn new(
        request: http::Request<Incoming>,
        params: PathParams,
        state: Arc<State>,
    ) -> Self {
        // Destructure the `http::Request` into its component parts.
        let (parts, body) = request.into_parts();
        // Check if the request has a `Content-Length` header. If it does, wrap
        // the request body in a `RequestBody` with a known length. Otherwise,
        // wrap the request body in a `RequestBody` with an unknown length.
        let body = AnyBody::Inline(body);

        Self {
            inner: Box::new(RequestData {
                state,
                params,
                parts,
                body,
            }),
        }
    }
}

impl<State> Debug for Request<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", self.method())
            .field("uri", self.uri())
            .field("params", &self.inner.params)
            .field("version", &self.version())
            .field("headers", self.headers())
            // .field("body", &self.data.body)
            .finish()
    }
}
