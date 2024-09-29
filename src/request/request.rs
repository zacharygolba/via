use bytes::Bytes;
use cookie::CookieJar;
use http::request::Parts;
use http::{HeaderMap, Method, Uri, Version};
use http_body::Body;
use hyper::body::Incoming;
use std::fmt::{self, Debug};
use std::sync::Arc;

use super::body::RequestBody;
use super::params::{Param, PathParams, QueryParam};
use crate::body::{AnyBody, BoxBody};
use crate::Error;

pub struct Request<State = ()> {
    /// The component parts of the underlying HTTP request.
    ///
    parts: Box<Parts>,

    /// A wrapper around the body of the request. This provides callers with
    /// convienent methods for reading the request body.
    ///
    body: RequestBody,

    /// The shared application state passed to the [`via::new`](crate::app::new)
    /// function.
    ///
    state: Arc<State>,

    /// The cookies associated with the request. If there is not a
    /// [CookieParser](crate::middleware::CookieParser)
    /// middleware in the middleware stack for the request, this will be empty.
    ///
    cookies: Option<Box<CookieJar>>,

    /// The request's path and query parameters.
    ///
    params: PathParams,
}

impl<State> Request<State> {
    /// Consumes the request and returns the body.
    ///
    pub fn into_body(self) -> RequestBody {
        self.body
    }

    /// Unwraps `self` into the Request type from the `http` crate.
    ///
    pub fn into_inner(self) -> http::Request<RequestBody> {
        let (parts, body) = self.into_parts();
        http::Request::from_parts(*parts, body)
    }

    /// Consumes the request and returns a tuple containing the component
    /// parts of the request and the request body.
    ///
    pub fn into_parts(self) -> (Box<Parts>, RequestBody) {
        (self.parts, self.body)
    }

    /// Consumes the request returning a new request with body mapped to the
    /// return type of the provided closure `map`.
    ///
    pub fn map<F, T, E>(self, map: F) -> Self
    where
        F: FnOnce(AnyBody<Incoming>) -> T,
        T: Body<Data = Bytes, Error = E> + Send + 'static,
        Error: From<E>,
    {
        let input = self.body.into_inner();
        let output = map(input);
        let box_body = AnyBody::Box(BoxBody::new(output));

        Self {
            body: RequestBody::new(box_body),
            ..self
        }
    }

    /// Returns a reference to the cookies associated with the request.
    ///
    pub fn cookies(&self) -> Option<&CookieJar> {
        self.cookies.as_deref()
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    ///
    pub fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    /// Returns a reference to the HTTP method associated with the request.
    ///
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
    /// use std::borrow::Cow;
    ///
    /// async fn hello(request: Request, _: Next) -> Result<String, Error> {
    ///     let name: Result<Cow<str>, Error> = request.param("name").into_result();
    ///     Ok(format!("Hello, {}!", name?))
    /// }
    /// ```
    pub fn param<'a>(&self, name: &'a str) -> Param<'_, 'a> {
        // Get the path of the request's uri.
        let path = self.parts.uri.path();

        // Get an `Option<[usize; 2]>` that represents the start and end offset
        // of the path parameter with the provided `name` in the request's uri.
        let at = self.params.get(name).copied();

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
    ///     let name = request.param("name").into_result()?;
    ///
    ///     // Create a greeting message with the provided `name`.
    ///     let message = format!("Hello, {}!\n", name);
    ///
    ///     // Send a response with our greeting message, repeated `n` times.
    ///     Ok(message.repeat(n))
    /// }
    /// ```
    pub fn query<'a>(&self, name: &'a str) -> QueryParam<'_, 'a> {
        let query = self.parts.uri.query().unwrap_or("");
        QueryParam::new(name, query)
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the [`via::new`](crate::app::new)
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
}

impl<State> Request<State> {
    pub(crate) fn new(parts: Box<Parts>, body: RequestBody, state: Arc<State>) -> Self {
        Self {
            parts,
            body,
            state,
            cookies: None,
            params: PathParams::new(),
        }
    }

    /// Returns a mutable reference to the cookies associated with the request.
    pub(crate) fn cookies_mut(&mut self) -> &mut CookieJar {
        self.cookies.get_or_insert_with(Default::default)
    }

    /// Returns a mutable reference to the cookies associated with the request.
    pub(crate) fn params_mut(&mut self) -> &mut PathParams {
        &mut self.params
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
            .field("cookies", &self.cookies)
            .field("body", &self.body)
            .finish()
    }
}
