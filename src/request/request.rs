use bytes::Bytes;
use cookie::CookieJar;
use http::header::{AsHeaderName, CONTENT_LENGTH, TRANSFER_ENCODING};
use http::request::Parts;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use http_body::Body;
use std::sync::Arc;

use super::body::{DataAndTrailers, RequestBody};
use super::param::PathParams;
use super::param::{PathParam, QueryParam};
use crate::error::{BoxError, Error};
use crate::response::{Finalize, Response, ResponseBuilder};

#[derive(Debug)]
pub struct Request<State = ()> {
    head: RequestHead<State>,
    body: RequestBody,
}

/// The component parts of a HTTP request.
///
#[derive(Debug)]
pub struct RequestHead<State> {
    pub(crate) parts: Parts,

    /// The request's path parameters.
    ///
    pub(crate) params: PathParams,

    /// The cookies associated with the request. If there is not a
    /// [CookieParser](crate::middleware::CookieParser)
    /// middleware in the middleware stack for the request, this will be empty.
    ///
    pub(crate) cookies: CookieJar,

    /// The shared application state passed to the
    /// [`App`](crate::App)
    /// constructor.
    ///
    pub(crate) state: Arc<State>,
}

impl<State> Request<State> {
    /// Returns a reference to the request's method.
    ///
    #[inline]
    pub fn method(&self) -> &Method {
        self.head.method()
    }

    /// Returns a reference to the request's URI.
    ///
    #[inline]
    pub fn uri(&self) -> &Uri {
        self.head.uri()
    }

    /// Returns the HTTP version that was used to make the request.
    ///
    #[inline]
    pub fn version(&self) -> Version {
        self.head.version()
    }

    /// Returns a reference to the request's headers.
    ///
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.head.headers()
    }

    /// Returns a reference to the associated extensions.
    ///
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        self.head.extensions()
    }

    /// Returns a mutable reference to the associated extensions.
    ///
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.head.extensions_mut()
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
    pub fn param<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
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
    pub fn query<'b>(&self, name: &'b str) -> QueryParam<'_, 'b> {
        self.head.query(name)
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
    #[inline]
    pub fn header<K>(&self, key: K) -> Result<Option<&str>, Error>
    where
        K: AsHeaderName,
    {
        self.head.header(key)
    }

    /// Returns reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        self.head.cookies()
    }

    /// Returns a reference to an [`Arc`] that contains the state argument that
    /// was passed as an argument to the [`App`](crate::App) constructor.
    ///
    #[inline]
    pub fn state(&self) -> &Arc<State> {
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
    pub fn into_parts(self) -> (RequestHead<State>, RequestBody) {
        (self.head, self.body)
    }

    /// Consumes the request and returns a future that resolves with the data
    /// in the body.
    ///
    pub async fn into_future(self) -> Result<DataAndTrailers, Error> {
        self.body.into_future().await
    }
}

impl<State> Request<State> {
    #[inline]
    pub(crate) fn new(head: RequestHead<State>, body: RequestBody) -> Self {
        Self { head, body }
    }

    /// Returns a mutable reference to the associated path params.
    ///
    #[inline]
    pub(crate) fn head_mut(&mut self) -> &mut RequestHead<State> {
        &mut self.head
    }
}

impl<State> Finalize for Request<State> {
    #[inline]
    fn finalize(self, builder: ResponseBuilder) -> Result<Response, Error> {
        let response = match self.headers().get(CONTENT_LENGTH).cloned() {
            Some(header_value) => builder.header(CONTENT_LENGTH, header_value),
            None => builder.header(TRANSFER_ENCODING, "chunked"),
        };

        response.body(self.body.boxed())
    }
}

impl<State> RequestHead<State> {
    #[inline]
    pub(crate) fn new(parts: Parts, state: Arc<State>, params: PathParams) -> Self {
        Self {
            parts,
            params,
            cookies: CookieJar::new(),
            state,
        }
    }

    /// Returns a reference to the request's method.
    ///
    #[inline]
    pub fn method(&self) -> &Method {
        &self.parts.method
    }

    /// Returns a reference to the request's URI.
    ///
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.parts.uri
    }

    /// Returns the HTTP version that was used to make the request.
    ///
    #[inline]
    pub fn version(&self) -> Version {
        self.parts.version
    }

    /// Returns a reference to the request's headers.
    ///
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    /// Returns a reference to the associated extensions.
    ///
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.parts.extensions
    }

    /// Returns a mutable reference to the associated extensions.
    ///
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.parts.extensions
    }

    /// Returns a convenient wrapper around an optional reference to the path
    /// parameter in the request's uri with the provided `name`.
    ///
    #[inline]
    pub fn param<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.params.get(self.uri().path(), name)
    }

    /// Returns a convenient wrapper around an optional references to the query
    /// parameters in the request's uri with the provided `name`.
    ///
    #[inline]
    pub fn query<'b>(&self, name: &'b str) -> QueryParam<'_, 'b> {
        QueryParam::new(name, self.uri().query().unwrap_or(""))
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
        self.headers()
            .get(key)
            .map(|value| value.to_str())
            .transpose()
            .map_err(|error| crate::err!(400, error))
    }

    /// Returns reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        &self.cookies
    }

    /// Returns a reference to an [`Arc`] that contains the state argument that
    /// was passed as an argument to the [`App`](crate::App) constructor.
    ///
    #[inline]
    pub fn state(&self) -> &Arc<State> {
        &self.state
    }
}
