use cookie::{Cookie, CookieJar};
use http::header::AsHeaderName;
use http::request::Parts;
use http::{HeaderMap, HeaderValue, Method, Uri, Version};
use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, Weak};

use super::param::{PathParam, PathParams};
use super::QueryParam;
use crate::body::{BoxBody, HttpBody, RequestBody};
use crate::Error;

pub struct Request<T = ()> {
    /// The shared application state passed to the
    /// [`via::app`](crate::app::app)
    /// function.
    ///
    state: Weak<T>,

    /// The component parts of the HTTP request.
    ///
    head: Parts,

    /// A length-limited, mappable wrapper around [hyper::body::Incoming].
    ///
    body: HttpBody<RequestBody>,

    /// The cookies associated with the request. If there is not a
    /// [CookieParser](crate::middleware::CookieParser)
    /// middleware in the middleware stack for the request, this will be empty.
    ///
    cookies: Option<Box<CookieJar>>,

    /// The request's path and query parameters.
    ///
    params: PathParams,
}

/// A wrapper around a weak reference to the state argument passed to
/// [`via::app`](crate::app::app).
///
pub struct State<T> {
    ptr: Weak<T>,
}

impl<T> Request<T> {
    /// Consumes the request returning a new request with body mapped to the
    /// return type of the provided closure `map`.
    ///
    #[inline]
    pub fn map(self, map: impl FnOnce(HttpBody<RequestBody>) -> BoxBody) -> Self {
        if cfg!(debug_assertions) && matches!(&self.body, HttpBody::Mapped(_)) {
            // TODO: Replace this with tracing and a proper logger.
            eprintln!("calling request.map() more than once can create a reference cycle.",);
        }

        Self {
            body: HttpBody::Mapped(map(self.body)),
            ..self
        }
    }

    /// Consumes the request and returns the body.
    ///
    #[inline]
    pub fn into_body(self) -> HttpBody<RequestBody> {
        self.body
    }

    /// Consumes the request and returns a tuple containing the component
    /// parts of the request and the request body.
    ///
    #[inline]
    pub fn into_parts(self) -> (Parts, HttpBody<RequestBody>) {
        (self.head, self.body)
    }

    /// Returns a reference to the body associated with the request.
    ///
    #[inline]
    pub fn body(&self) -> &HttpBody<RequestBody> {
        &self.body
    }

    /// Returns an optional reference to the cookie with the provided `name`.
    ///
    #[inline]
    pub fn cookie(&self, name: &str) -> Option<&Cookie<'static>> {
        self.cookies()?.get(name)
    }

    /// Returns an optional reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> Option<&CookieJar> {
        self.cookies.as_deref()
    }

    /// Returns a reference to the header value associated with the key.
    ///
    #[inline]
    pub fn header<K: AsHeaderName>(&self, key: K) -> Option<&HeaderValue> {
        self.head.headers.get(key)
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    ///
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.head.headers
    }

    /// Returns a reference to the HTTP method associated with the request.
    ///
    #[inline]
    pub fn method(&self) -> &Method {
        &self.head.method
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
            self.head.uri.path(),
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
        QueryParam::new(name, self.head.uri.query().unwrap_or(""))
    }

    /// Returns a thread-safe reference-counting pointer to the application
    /// state that was passed as an argument to the
    /// [`via::app`](crate::app::app)
    /// function.
    ///
    #[inline]
    pub fn state(&self) -> State<T> {
        State {
            ptr: Weak::clone(&self.state),
        }
    }

    /// Returns a reference to the uri associated with the request.
    ///
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.head.uri
    }

    /// Returns the HTTP version associated with the request.
    ///
    #[inline]
    pub fn version(&self) -> Version {
        self.head.version
    }
}

impl<T> Request<T> {
    #[inline]
    pub(crate) fn new(
        state: Weak<T>,
        params: PathParams,
        head: Parts,
        body: HttpBody<RequestBody>,
    ) -> Self {
        Self {
            state,
            cookies: None,
            params,
            head,
            body,
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
            .field("version", &self.version())
            .field("method", self.method())
            .field("uri", self.uri())
            .field("headers", self.headers())
            .field("params", &self.params)
            .field("cookies", &self.cookies)
            .field("body", self.body())
            .finish()
    }
}

impl<T> State<T> {
    /// Attempt to upgrade the weak reference to a strong reference.
    ///
    pub fn upgrade(self) -> Option<Arc<T>> {
        Weak::upgrade(&self.ptr)
    }

    /// Attempt to upgrade the weak referene to a strong reference. If the
    /// value was dropped, an error is returned.
    ///
    pub fn try_upgrade(self) -> Result<Arc<T>, Error> {
        self.upgrade().ok_or_else(|| {
            let message = "the application state is unavailable".to_string();
            Error::internal_server_error(message.into())
        })
    }
}
