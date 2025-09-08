use bytes::Bytes;
use cookie::CookieJar;
use http::header::{AsHeaderName, CONTENT_LENGTH, TRANSFER_ENCODING};
use http::request::Parts;
use http::{HeaderMap, Method, Uri, Version};
use http_body::Body;
use std::sync::Arc;
use via_router::Param;

use super::body::RequestBody;
use super::param::{PathParam, QueryParam};
use crate::error::{BoxError, Error};
use crate::request::RequestPayload;
use crate::response::{Pipe, Response, ResponseBuilder};

/// The component parts of a HTTP request.
///
#[derive(Debug)]
pub struct RequestHead<T> {
    pub parts: Parts,

    /// The request's path parameters.
    ///
    pub(crate) params: Vec<(Arc<str>, Param)>,

    /// The cookies associated with the request. If there is not a
    /// [CookieParser](crate::middleware::CookieParser)
    /// middleware in the middleware stack for the request, this will be empty.
    ///
    cookies: CookieJar,

    /// The shared application state passed to the
    /// [`via::app`](crate::app::app)
    /// function.
    ///
    state: Arc<T>,
}

#[derive(Debug)]
pub struct Request<T = ()> {
    head: RequestHead<T>,
    body: RequestBody,
}

impl<T> RequestHead<T> {
    #[inline]
    pub(crate) fn new(parts: Parts, state: Arc<T>, params: Vec<(Arc<str>, Param)>) -> Self {
        Self {
            parts,
            params,
            cookies: CookieJar::new(),
            state,
        }
    }

    /// Returns a result that contains an Option<&str> with the header value
    /// associated with the provided key.
    ///
    /// # Errors
    ///
    /// *Status Code:* `400`
    ///
    /// If the header value associated with key contains a char that is not
    /// considered to be visible ascii.
    ///
    pub fn header<K>(&self, key: K) -> Result<Option<&str>, Error>
    where
        K: AsHeaderName,
    {
        self.parts
            .headers
            .get(key)
            .map(|value| value.to_str())
            .transpose()
            .map_err(Error::bad_request)
    }

    /// Returns a convenient wrapper around an optional reference to the path
    /// parameter in the request's uri with the provided `name`.
    ///
    #[inline]
    pub fn param<'a>(&self, name: &'a str) -> PathParam<'_, 'a> {
        PathParam::new(
            name,
            self.parts.uri.path(),
            self.params
                .iter()
                .find_map(|(n, range)| if &**n == name { Some(*range) } else { None }),
        )
    }

    /// Returns a convenient wrapper around an optional references to the query
    /// parameters in the request's uri with the provided `name`.
    ///
    #[inline]
    pub fn query<'a>(&self, name: &'a str) -> QueryParam<'_, 'a> {
        QueryParam::new(name, self.parts.uri.query().unwrap_or(""))
    }

    /// Returns reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        &self.cookies
    }

    #[inline]
    pub fn state(&self) -> &Arc<T> {
        &self.state
    }
}

impl<T> Request<T> {
    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    ///
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.head.parts.headers
    }

    /// Returns a result that contains an Option<&str> with the header value
    /// associated with the provided key.
    ///
    /// # Errors
    ///
    /// *Status Code:* `400`
    ///
    /// If the header value associated with key contains a char that is not
    /// considered to be visible ascii.
    ///
    pub fn header<K>(&self, key: K) -> Result<Option<&str>, Error>
    where
        K: AsHeaderName,
    {
        self.head.header(key)
    }

    /// Returns a reference to the HTTP method associated with the request.
    ///
    #[inline]
    pub fn method(&self) -> &Method {
        &self.head.parts.method
    }

    /// Returns a reference to the uri associated with the request.
    ///
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.head.parts.uri
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
        self.head.param(name)
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
        self.head.query(name)
    }

    /// Returns the HTTP version associated with the request.
    ///
    #[inline]
    pub fn version(&self) -> Version {
        self.head.parts.version
    }

    /// Returns reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        self.head.cookies()
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the
    /// [`via::app`](crate::app::app)
    /// function.
    ///
    #[inline]
    pub fn state(&self) -> &Arc<T> {
        self.head.state()
    }

    /// Consumes the request returning a new request with body mapped to the
    /// return type of the provided closure.
    ///
    #[inline]
    pub fn map<U, F>(self, map: F) -> Self
    where
        F: FnOnce(RequestBody) -> U,
        U: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self {
            body: self.body.map(map),
            ..self
        }
    }

    /// Consumes the request and returns a tuple containing the head and body.
    ///
    #[inline]
    pub fn into_parts(self) -> (RequestHead<T>, RequestBody) {
        (self.head, self.body)
    }

    /// Consumes the request and returns a future that resolves with the data
    /// in the body.
    ///
    pub async fn into_future(self) -> Result<RequestPayload, Error> {
        self.body.into_future().await
    }
}

impl<T> Request<T> {
    #[inline]
    pub(crate) fn new(head: RequestHead<T>, body: RequestBody) -> Self {
        Self { head, body }
    }

    /// Returns a mutable reference to the associated path params.
    ///
    #[inline]
    pub(crate) fn head_mut(&mut self) -> &mut RequestHead<T> {
        &mut self.head
    }

    /// Returns a mutable reference to the cookies associated with the request.
    ///
    #[inline]
    pub(crate) fn cookies_mut(&mut self) -> &mut CookieJar {
        &mut self.head.cookies
    }
}

impl<T> Pipe for Request<T> {
    fn pipe(self, builder: ResponseBuilder) -> Result<Response, Error> {
        let response = match self.headers().get(CONTENT_LENGTH) {
            Some(len) => builder.header(CONTENT_LENGTH, len),
            None => builder.header(TRANSFER_ENCODING, "chunked"),
        };

        response.body(self.body.boxed())
    }
}
