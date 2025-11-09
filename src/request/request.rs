use cookie::CookieJar;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use http_body_util::Limited;
use hyper::body::Incoming as Body;
use std::fmt::{self, Debug, Formatter};

use super::body::IntoFuture;
use super::params::{Param, PathParamEntry, PathParams};
use crate::app::Shared;
use crate::error::Error;
use crate::request::params::QueryParams;
use crate::response::{Finalize, Response, ResponseBuilder};

pub struct Head {
    pub(crate) parts: http::request::Parts,
    pub(crate) params: Vec<PathParamEntry>,
    pub(crate) cookies: CookieJar,
}

#[derive(Debug)]
pub struct Parts<State> {
    head: Box<Head>,
    state: Shared<State>,
}

pub struct Request<State = ()> {
    head: Box<Head>,
    body: Limited<Body>,
    state: Shared<State>,
}

impl Head {
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

    /// Returns reference to the cookies associated with the request.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        &self.cookies
    }

    pub fn params<'a, T>(&'a self) -> crate::Result<T>
    where
        T: TryFrom<PathParams<'a>, Error = Error>,
    {
        T::try_from(PathParams::new(self.uri().path(), &self.params))
    }

    /// Returns a convenient wrapper around an optional reference to the path
    /// parameter in the request's uri with the provided `name`.
    ///
    pub fn param<'b>(&self, name: &'b str) -> Param<'_, 'b> {
        PathParams::new(self.uri().path(), &self.params).get(name)
    }

    pub fn query<'a, T>(&'a self) -> crate::Result<T>
    where
        T: TryFrom<QueryParams<'a>, Error = Error>,
    {
        T::try_from(QueryParams::new(self.uri().query()))
    }
}

impl Debug for Head {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Head")
            .field("method", self.method())
            .field("uri", self.uri())
            .field("params", &self.params)
            .field("version", &self.version())
            .field("headers", self.headers())
            .field("cookies", self.cookies())
            .field("extensions", self.extensions())
            .finish()
    }
}

impl<State> Parts<State> {
    #[inline]
    pub fn head(&self) -> &Head {
        &self.head
    }

    #[inline]
    pub fn head_mut(&mut self) -> &mut Head {
        &mut self.head
    }

    #[inline]
    pub fn state(&self) -> &Shared<State> {
        &self.state
    }
}

impl<State> Request<State> {
    #[inline]
    pub(crate) fn new(
        state: Shared<State>,
        parts: http::request::Parts,
        body: Limited<Body>,
    ) -> Self {
        let head = Box::new(Head {
            parts,
            params: Vec::with_capacity(8),
            cookies: CookieJar::new(),
        });

        Self { state, head, body }
    }

    #[inline]
    pub fn head(&self) -> &Head {
        &self.head
    }

    #[inline]
    pub fn head_mut(&mut self) -> &mut Head {
        &mut self.head
    }

    #[inline]
    pub fn state(&self) -> &Shared<State> {
        &self.state
    }

    /// Consumes the request and returns a future that resolves with the data
    /// in the body.
    ///
    #[inline]
    pub fn into_future(self) -> (Shared<State>, IntoFuture) {
        let Self { body, state, .. } = self;
        (state, IntoFuture::new(body))
    }

    /// Consumes the request and returns a tuple containing the head and body.
    ///
    #[inline]
    pub fn into_parts(self) -> (Parts<State>, Limited<Body>) {
        let Self { head, body, state } = self;
        (Parts { head, state }, body)
    }
}

impl<State> Debug for Request<State> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Request")
            .field("head", self.head())
            .field("body", &self.body)
            .field("state", self.state())
            .finish()
    }
}

impl<State> Finalize for Request<State> {
    fn finalize(self, response: ResponseBuilder) -> Result<Response, Error> {
        use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
        use http_body_util::combinators::BoxBody;

        let headers = self.head().headers();

        let mut response = match headers.get(CONTENT_LENGTH).cloned() {
            Some(content_length) => response.header(CONTENT_LENGTH, content_length),
            None => response.header(TRANSFER_ENCODING, "chunked"),
        };

        if let Some(content_type) = headers.get(CONTENT_TYPE).cloned() {
            response = response.header(CONTENT_TYPE, content_type);
        }

        response.body(BoxBody::new(self.body).into())
    }
}
