use bytes::Bytes;
use cookie::CookieJar;
use http::request::Parts;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use http_body::Body;
use http_body_util::Either;
use http_body_util::combinators::BoxBody;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use super::body::{IntoFuture, RequestBody};
use super::params::{Param, PathParamEntry, PathParams};
use crate::error::{BoxError, Error};
use crate::request::params::QueryParams;
use crate::response::{Finalize, Response, ResponseBuilder};

/// The component parts of a HTTP request.
///
pub struct RequestHead<State> {
    envelope: Box<Envelope>,
    state: Arc<State>,
}

pub struct Request<State = ()> {
    head: RequestHead<State>,
    body: RequestBody,
}

#[derive(Debug)]
pub(crate) struct Envelope {
    pub(crate) parts: Parts,
    pub(crate) params: Vec<PathParamEntry>,
    pub(crate) cookies: CookieJar,
}

impl<State> RequestHead<State> {
    pub(crate) fn new(parts: Parts, state: Arc<State>) -> Self {
        let envelope = Box::new(Envelope {
            parts,
            params: Vec::with_capacity(8),
            cookies: CookieJar::new(),
        });

        Self { envelope, state }
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
}

impl<State> Debug for RequestHead<State> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[derive(Debug)]
        pub struct State;

        f.debug_struct("RequestHead")
            .field("envelope", &*self.envelope)
            .field("state", &State)
            .finish()
    }
}

impl<State> Request<State> {
    #[inline]
    pub(crate) fn new(head: RequestHead<State>, body: RequestBody) -> Self {
        Self { head, body }
    }

    #[inline]
    pub(crate) fn envelope_mut(&mut self) -> &mut Envelope {
        &mut self.head.envelope
    }

    #[inline]
    pub fn head(&self) -> &RequestHead<State> {
        &self.head
    }

    #[inline]
    pub fn body(&self) -> &RequestBody {
        &self.body
    }

    /// Returns a reference to an [`Arc`] that contains the state argument that
    /// was passed as an argument to the [`App`](crate::App) constructor.
    ///
    #[inline]
    pub fn state(&self) -> &Arc<State> {
        &self.head.state
    }

    /// Returns a mutable reference to the associated extensions.
    ///
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.head.extensions_mut()
    }

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
    pub fn map<R, F>(self, map: F) -> Self
    where
        R: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
        F: FnOnce(RequestBody) -> R,
    {
        Self {
            head: self.head,
            body: RequestBody {
                kind: Either::Right(BoxBody::new(map(self.body))),
            },
        }
    }
}

impl<State> Debug for Request<State> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Request")
            .field("head", self.head())
            .field("body", self.body())
            .finish()
    }
}

impl<State> Finalize for Request<State> {
    fn finalize(self, response: ResponseBuilder) -> Result<Response, Error> {
        use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};

        let Self { ref head, body } = self;
        let headers = head.headers();
        let body = match body.kind {
            Either::Left(inline) => BoxBody::new(inline),
            Either::Right(boxed) => boxed,
        };

        let mut response = match headers.get(CONTENT_LENGTH).cloned() {
            Some(content_length) => response.header(CONTENT_LENGTH, content_length),
            None => response.header(TRANSFER_ENCODING, "chunked"),
        };

        if let Some(content_type) = headers.get(CONTENT_TYPE).cloned() {
            response = response.header(CONTENT_TYPE, content_type);
        }

        response.body(body)
    }
}
