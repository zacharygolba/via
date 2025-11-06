use bytes::Bytes;
use cookie::CookieJar;
use http::header::{self, CONTENT_LENGTH, TRANSFER_ENCODING};
use http::request::Parts;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use http_body::Body;
use std::sync::Arc;

use super::body::{IntoFuture, RequestBody};
use super::params::{Param, PathParamEntry, PathParams};
use crate::error::{BoxError, Error};
use crate::request::params::QueryParams;
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
    envelope: Box<Envelope>,
    state: Arc<State>,
}

#[derive(Debug)]
struct Envelope {
    parts: Parts,
    params: Vec<PathParamEntry>,
    cookies: CookieJar,
}

impl<State> Request<State> {
    /// Consumes the request and returns a future that resolves with the data
    /// in the body.
    ///
    #[inline]
    pub fn into_future(self) -> (RequestHead<State>, IntoFuture) {
        (self.head, self.body.into_future())
    }

    /// Consumes the request and returns a tuple containing the head and body.
    ///
    #[inline]
    pub fn into_parts(self) -> (RequestHead<State>, RequestBody) {
        (self.head, self.body)
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

    #[inline]
    pub fn head(&self) -> &RequestHead<State> {
        &self.head
    }

    #[inline]
    pub fn head_mut(&mut self) -> &mut RequestHead<State> {
        &mut self.head
    }

    /// Returns a reference to an [`Arc`] that contains the state argument that
    /// was passed as an argument to the [`App`](crate::App) constructor.
    ///
    #[inline]
    pub fn state(&self) -> &Arc<State> {
        &self.head.state
    }
}

impl<State> Request<State> {
    #[inline]
    pub(crate) fn new(head: RequestHead<State>, body: RequestBody) -> Self {
        Self { head, body }
    }

    pub(crate) fn cookie_str_with_cookies_mut(&mut self) -> Option<(&str, &mut CookieJar)> {
        let envelope = &mut *self.head.envelope;
        let header = envelope.parts.headers.get(header::COOKIE)?.to_str().ok()?;

        Some((header, &mut envelope.cookies))
    }

    pub(crate) fn path_with_params_mut(&mut self) -> (&str, &mut Vec<PathParamEntry>) {
        let envelope = &mut *self.head.envelope;
        (envelope.parts.uri.path(), &mut envelope.params)
    }
}

impl<State> Finalize for Request<State> {
    fn finalize(self, builder: ResponseBuilder) -> Result<Response, Error> {
        let Self { head, body } = self;
        let body = body.boxed();

        if let Some(value) = head.headers().get(CONTENT_LENGTH) {
            builder.header(CONTENT_LENGTH, value).body(body)
        } else {
            builder.header(TRANSFER_ENCODING, "chunked").body(body)
        }
    }
}

impl<State> RequestHead<State> {
    pub(crate) fn new(parts: Parts, state: Arc<State>) -> Self {
        Self {
            envelope: Box::new(Envelope {
                cookies: CookieJar::new(),
                params: Vec::with_capacity(8),
                parts,
            }),
            state,
        }
    }

    /// Returns a reference to the request's method.
    ///
    #[inline]
    pub fn method(&self) -> &Method {
        &self.envelope.parts.method
    }

    /// Returns a reference to the request's URI.
    ///
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.envelope.parts.uri
    }

    /// Returns the HTTP version that was used to make the request.
    ///
    #[inline]
    pub fn version(&self) -> Version {
        self.envelope.parts.version
    }

    /// Returns a reference to the request's headers.
    ///
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.envelope.parts.headers
    }

    /// Returns a reference to the associated extensions.
    ///
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.envelope.parts.extensions
    }

    /// Returns a mutable reference to the associated extensions.
    ///
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.envelope.parts.extensions
    }

    pub fn params<'a, T>(&'a self) -> crate::Result<T>
    where
        T: TryFrom<PathParams<'a>, Error = Error>,
    {
        let envelope = &*self.envelope;
        let params = PathParams::new(envelope.parts.uri.path(), &envelope.params);

        T::try_from(params)
    }

    /// Returns a convenient wrapper around an optional reference to the path
    /// parameter in the request's uri with the provided `name`.
    ///
    pub fn param<'b>(&self, name: &'b str) -> Param<'_, 'b> {
        let envelope = &*self.envelope;
        let params = PathParams::new(envelope.parts.uri.path(), &envelope.params);

        params.get(name)
    }

    pub fn query<'a, T>(&'a self) -> crate::Result<T>
    where
        T: TryFrom<QueryParams<'a>, Error = Error>,
    {
        T::try_from(QueryParams::new(self.uri().query()))
    }

    /// Returns reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        &self.envelope.cookies
    }

    /// Returns a reference to an [`Arc`] that contains the state argument that
    /// was passed as an argument to the [`App`](crate::App) constructor.
    ///
    #[inline]
    pub fn state(&self) -> &State {
        &self.state
    }
}
