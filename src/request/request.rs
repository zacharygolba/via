use cookie::CookieJar;
use http::header::{AsHeaderName, CONTENT_LENGTH, TRANSFER_ENCODING};
use http::request::Parts;
use http::{HeaderMap, Method, Uri, Version};
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use super::param::{PathParam, PathParams, QueryParam};
use crate::body::{BodyReader, BodyStream, BoxBody};
use crate::response::ResponseBuilder;
use crate::{Error, Pipe, Response};

/// The component parts of a HTTP request.
///
pub struct Head {
    pub parts: Parts,

    /// The cookies associated with the request. If there is not a
    /// [CookieParser](crate::middleware::CookieParser)
    /// middleware in the middleware stack for the request, this will be empty.
    ///
    cookies: CookieJar,

    /// The request's path and query parameters.
    ///
    params: PathParams,
}

pub struct Request<T = ()> {
    /// The shared application state passed to the
    /// [`via::app`](crate::app::app)
    /// function.
    ///
    state: Arc<T>,

    head: Box<Head>,

    body: BoxBody,
}

impl Head {
    #[inline]
    pub(crate) fn new(parts: Parts, params: PathParams) -> Self {
        Self {
            parts,
            params,
            cookies: CookieJar::new(),
        }
    }

    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        &self.cookies
    }

    /// Returns a convenient wrapper around an optional reference to the path
    /// parameter in the request's uri with the provided `name`.
    ///
    #[inline]
    pub fn param<'a>(&self, name: &'a str) -> PathParam<'_, 'a> {
        PathParam::new(
            name,
            self.parts.uri.path(),
            Some(self.params.iter().find_map(
                |(param, range)| {
                    if param == name {
                        Some(*range)
                    } else {
                        None
                    }
                },
            )),
        )
    }

    /// Returns a convenient wrapper around an optional references to the query
    /// parameters in the request's uri with the provided `name`.
    ///
    #[inline]
    pub fn query<'a>(&self, name: &'a str) -> QueryParam<'_, 'a> {
        QueryParam::new(name, self.parts.uri.query().unwrap_or(""))
    }
}

impl<T> Request<T> {
    /// Consumes the request returning a new request with body mapped to the
    /// return type of the provided closure `map`.
    ///
    #[inline]
    pub fn map<F>(self, map: F) -> Self
    where
        F: FnOnce(BoxBody) -> BoxBody,
    {
        Self {
            body: map(self.body),
            ..self
        }
    }

    #[inline]
    pub fn into_body(self) -> BoxBody {
        self.body
    }

    #[inline]
    pub fn into_parts(self) -> (Box<Head>, BoxBody) {
        (self.head, self.body)
    }

    pub fn into_future(self) -> BodyReader {
        BodyReader::new(self.body)
    }

    pub fn into_stream(self) -> BodyStream {
        BodyStream::new(self.body)
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
        match self.headers().get(key).map(|value| value.to_str()) {
            Some(Ok(value_as_str)) => Ok(Some(value_as_str)),
            Some(Err(error)) => Err(Error::bad_request(error.into())),
            None => Ok(None),
        }
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    ///
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.head.parts.headers
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
    pub fn query<'a>(&self, name: &'a str) -> QueryParam<'_, 'a> {
        self.head.query(name)
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

    /// Returns an optional reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        &self.head.cookies
    }

    /// Returns the HTTP version associated with the request.
    ///
    #[inline]
    pub fn version(&self) -> Version {
        self.head.parts.version
    }
}

impl<T> Request<T> {
    #[inline]
    pub(crate) fn new(state: Arc<T>, head: Box<Head>, body: BoxBody) -> Self {
        Self { state, head, body }
    }

    /// Returns a mutable reference to the cookies associated with the request.
    ///
    #[inline]
    pub(crate) fn cookies_mut(&mut self) -> &mut CookieJar {
        &mut self.head.cookies
    }
}

impl<T> Debug for Request<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Request")
            .field("version", &self.version())
            .field("method", self.method())
            .field("uri", self.uri())
            .field("headers", self.headers())
            .field("params", &self.head.params)
            .field("cookies", &self.head.cookies)
            .field("body", &self.body)
            .finish()
    }
}

impl<T> Pipe for Request<T> {
    fn pipe(self, builder: ResponseBuilder) -> Result<Response, Error> {
        let response = match self.headers().get(CONTENT_LENGTH) {
            Some(len) => builder.header(CONTENT_LENGTH, len),
            None => builder.header(TRANSFER_ENCODING, "chunked"),
        };

        response.boxed(self.body)
    }
}
