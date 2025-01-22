use cookie::{Cookie, CookieJar};
use http::header::AsHeaderName;
use http::request::Parts;
use http::{HeaderMap, HeaderValue, Method, Uri, Version};
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use super::param::{PathParam, PathParams};
use super::QueryParam;
use crate::body::{BoxBody, HttpBody, RequestBody};

pub struct Request<T = ()> {
    /// The shared application state passed to the
    /// [`via::app`](crate::app::app)
    /// function.
    ///
    state: Arc<T>,

    /// The cookies associated with the request. If there is not a
    /// [CookieParser](crate::middleware::CookieParser)
    /// middleware in the middleware stack for the request, this will be empty.
    ///
    cookies: Option<CookieJar>,

    /// The request's path and query parameters.
    ///
    params: PathParams,

    /// A wrapper around the body of the request. This provides callers with
    /// convienent methods for reading the request body.
    ///
    inner: http::Request<HttpBody<RequestBody>>,
}

impl<T> Request<T> {
    /// Consumes the request returning a new request with body mapped to the
    /// return type of the provided closure `map`.
    ///
    #[inline]
    pub fn map(self, map: impl FnOnce(HttpBody<RequestBody>) -> BoxBody) -> Self {
        if cfg!(debug_assertions) && matches!(self.inner.body(), HttpBody::Mapped(_)) {
            // TODO: Replace this with tracing and a proper logger.
            eprintln!("calling request.map() more than once can create a reference cycle.",);
        }

        Self {
            inner: self.inner.map(|body| HttpBody::Mapped(map(body))),
            ..self
        }
    }

    /// Consumes the request and returns the body.
    ///
    #[inline]
    pub fn into_body(self) -> HttpBody<RequestBody> {
        self.inner.into_body()
    }

    /// Consumes the request and returns a tuple containing the component
    /// parts of the request and the request body.
    ///
    #[inline]
    pub fn into_parts(self) -> (Parts, HttpBody<RequestBody>) {
        self.inner.into_parts()
    }

    /// Returns a reference to the body associated with the request.
    ///
    #[inline]
    pub fn body(&self) -> &HttpBody<RequestBody> {
        self.inner.body()
    }

    /// Returns an optional reference to the cookie with the provided `name`.
    ///
    #[inline]
    pub fn cookie(&self, name: &str) -> Option<&Cookie<'static>> {
        self.cookies.as_ref()?.get(name)
    }

    /// Returns an optional reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> Option<&CookieJar> {
        self.cookies.as_ref()
    }

    /// Returns a reference to the header value associated with the key.
    ///
    #[inline]
    pub fn header<K: AsHeaderName>(&self, key: K) -> Option<&HeaderValue> {
        self.inner.headers().get(key)
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    ///
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// Returns a reference to the HTTP method associated with the request.
    ///
    #[inline]
    pub fn method(&self) -> &Method {
        self.inner.method()
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
    #[inline]
    pub fn param<'a>(&self, name: &'a str) -> PathParam<'_, 'a> {
        PathParam::new(
            name,
            self.inner.uri().path(),
            self.params.iter().rev().find_map(
                |(param, at)| {
                    if param == name {
                        Some(*at)
                    } else {
                        None
                    }
                },
            ),
        )
    }

    /// Returns a convenient wrapper around an optional references to the query
    /// parameters in the request's uri with the provided `name`.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{Next, Request, Response};
    ///
    /// async fn hello(request: Request, _: Next) -> via::Result {
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
    ///     Response::build().text(message.repeat(n))
    /// }
    /// ```
    ///
    #[inline]
    pub fn query<'a>(&self, name: &'a str) -> QueryParam<'_, 'a> {
        QueryParam::new(name, self.inner.uri().query().unwrap_or(""))
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the
    /// [`via::app`](crate::app::app)
    /// function.
    ///
    #[inline]
    pub fn state(&self) -> &Arc<T> {
        &self.state
    }

    /// Returns a reference to the uri associated with the request.
    ///
    #[inline]
    pub fn uri(&self) -> &Uri {
        self.inner.uri()
    }

    /// Returns the HTTP version associated with the request.
    ///
    #[inline]
    pub fn version(&self) -> Version {
        self.inner.version()
    }
}

impl<T> Request<T> {
    #[inline]
    pub(crate) fn new(
        state: Arc<T>,
        params: PathParams,
        request: http::Request<HttpBody<RequestBody>>,
    ) -> Self {
        Self {
            state,
            cookies: None,
            params,
            inner: request,
        }
    }

    /// Returns a mutable reference to the cookies associated with the request.
    ///
    #[inline]
    pub(crate) fn cookies_mut(&mut self) -> &mut CookieJar {
        self.cookies.get_or_insert_default()
    }
}

impl<T> Debug for Request<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Request")
            .field("version", &self.inner.version())
            .field("method", self.inner.method())
            .field("uri", self.inner.uri())
            .field("headers", self.inner.headers())
            .field("params", &self.params)
            .field("cookies", &self.cookies)
            .field("body", self.inner.body())
            .finish()
    }
}
