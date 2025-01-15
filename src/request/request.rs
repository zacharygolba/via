use cookie::{Cookie, CookieJar};
use http::header::AsHeaderName;
use http::request::Parts;
use http::{HeaderMap, HeaderValue, Method, Uri, Version};
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use super::body::RequestBody;
use super::param::{PathParam, PathParams};
use crate::body::{BoxBody, HttpBody};

pub struct Request<T = ()> {
    mapped: bool,

    /// The component parts of the underlying HTTP request.
    ///
    pub(crate) parts: Box<Parts>,

    /// A wrapper around the body of the request. This provides callers with
    /// convienent methods for reading the request body.
    ///
    body: HttpBody<RequestBody>,

    /// The shared application state passed to the [`via::new`](crate::app::new)
    /// function.
    ///
    state: Arc<T>,

    /// The cookies associated with the request. If there is not a
    /// [CookieParser](crate::middleware::CookieParser)
    /// middleware in the middleware stack for the request, this will be empty.
    ///
    cookies: Option<Box<CookieJar>>,

    /// The request's path and query parameters.
    ///
    pub(crate) params: PathParams,
}

impl<T> Request<T> {
    /// Consumes the request and returns the body.
    ///
    pub fn into_body(self) -> HttpBody<RequestBody> {
        self.body
    }

    /// Consumes the request returning a new request with body mapped to the
    /// return type of the provided closure `map`.
    ///
    pub fn map(self, map: impl FnOnce(HttpBody<RequestBody>) -> BoxBody) -> Self {
        if cfg!(debug_assertions) && self.mapped {
            // TODO: Replace this with tracing and a proper logger.
            eprintln!("calling request.map() more than once can create a reference cycle.",);
        }

        Self {
            mapped: true,
            body: HttpBody::Box(map(self.body)),
            ..self
        }
    }

    /// Returns an optional reference to the cookie with the provided `name`.
    ///
    pub fn cookie(&self, name: &str) -> Option<&Cookie<'static>> {
        self.cookies.as_ref()?.get(name)
    }

    /// Returns an optional reference to the cookies associated with the request.
    ///
    pub fn cookies(&self) -> Option<&CookieJar> {
        self.cookies.as_deref()
    }

    /// Returns a reference to the header value associated with the key.
    ///
    pub fn header<K: AsHeaderName>(&self, key: K) -> Option<&HeaderValue> {
        self.parts.headers.get(key)
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
    /// use via::{Next, Request, Response};
    ///
    /// async fn hello(request: Request, _: Next) -> via::Result {
    ///     let name = request.param("name").into_result()?;
    ///     Response::build().text(format!("Hello, {}!", name))
    /// }
    /// ```
    ///
    pub fn param<'a>(&self, name: &'a str) -> PathParam<'_, 'a> {
        let path = self.parts.uri.path();
        let at = self.params.iter().find_map(|(param, span)| {
            if name == param {
                Some((span.start(), span.end()))
            } else {
                None
            }
        });

        PathParam::new(at, name, path)
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the [`via::new`](crate::app::new)
    /// function.
    ///
    pub fn state(&self) -> &Arc<T> {
        &self.state
    }

    /// Returns a reference to the uri associated with the request.
    ///
    pub fn uri(&self) -> &Uri {
        &self.parts.uri
    }

    /// Returns the HTTP version associated with the request.
    ///
    pub fn version(&self) -> Version {
        self.parts.version
    }

    /// Consumes the request and returns a tuple containing the component
    /// parts of the request and the request body.
    ///
    pub fn into_parts(self) -> (Box<Parts>, HttpBody<RequestBody>) {
        (self.parts, self.body)
    }

    /// Unwraps `self` into the Request type from the `http` crate.
    ///
    pub fn into_inner(self) -> http::Request<HttpBody<RequestBody>> {
        let (parts, body) = self.into_parts();
        http::Request::from_parts(*parts, body)
    }
}

impl<T> Request<T> {
    pub(crate) fn new(parts: Box<Parts>, body: HttpBody<RequestBody>, state: Arc<T>) -> Self {
        Self {
            parts,
            body,
            state,
            mapped: false,
            cookies: None,
            params: PathParams::new(Vec::with_capacity(8)),
        }
    }

    /// Returns a mutable reference to the cookies associated with the request.
    ///
    pub(crate) fn cookies_mut(&mut self) -> &mut CookieJar {
        self.cookies.get_or_insert_default()
    }
}

impl<T> Debug for Request<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
