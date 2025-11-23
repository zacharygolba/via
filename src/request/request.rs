use cookie::CookieJar;
use http::request::Parts;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use http_body_util::Limited;
use hyper::body::Incoming as Body;
use std::fmt::{self, Debug, Formatter};

use super::params::{Param, PathParamEntry, PathParams};
use super::payload::IntoFuture;
use crate::app::Shared;
use crate::error::Error;
use crate::request::params::QueryParams;
use crate::response::{Finalize, Response, ResponseBuilder};

pub struct Envelope {
    pub(crate) parts: Parts,
    pub(crate) params: Vec<PathParamEntry>,
    pub(crate) cookies: CookieJar,
}

pub struct Request<App = ()> {
    envelope: Box<Envelope>,
    body: Limited<Body>,
    app: Shared<App>,
}

impl Envelope {
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

impl Debug for Envelope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[derive(Debug)]
        struct CookieJar;

        f.debug_struct("Envelope")
            .field("method", self.method())
            .field("uri", self.uri())
            .field("params", &self.params)
            .field("version", &self.version())
            .field("headers", self.headers())
            .field("cookies", &CookieJar)
            .field("extensions", self.extensions())
            .finish()
    }
}

impl<App> Request<App> {
    #[inline]
    pub(crate) fn new(app: Shared<App>, parts: Parts, body: Limited<Body>) -> Self {
        let envelope = Box::new(Envelope {
            parts,
            params: Vec::new(),
            cookies: CookieJar::new(),
        });

        Self {
            envelope,
            body,
            app,
        }
    }

    #[inline]
    pub fn app(&self) -> &Shared<App> {
        &self.app
    }

    #[inline]
    pub fn envelope(&self) -> &Envelope {
        &self.envelope
    }

    #[inline]
    pub fn envelope_mut(&mut self) -> &mut Envelope {
        &mut self.envelope
    }

    /// Consumes the request and returns a tuple containing a future that
    /// resolves with the data and trailers of the body as well as a shared
    /// copy of `State`.
    ///
    #[inline]
    pub fn into_future(self) -> (IntoFuture, Shared<App>) {
        let Self { app, body, .. } = self;
        (IntoFuture::new(body), app)
    }

    /// Consumes the request and returns a tuple containing it's parts.
    ///
    #[inline]
    pub fn into_parts(self) -> (Box<Envelope>, Limited<Body>, Shared<App>) {
        (self.envelope, self.body, self.app)
    }
}

impl<State> Debug for Request<State> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Request")
            .field("envelope", self.envelope())
            .field("body", &self.body)
            .field("app", self.app())
            .finish()
    }
}

impl<State> Finalize for Request<State> {
    fn finalize(self, response: ResponseBuilder) -> Result<Response, Error> {
        use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
        use http_body_util::combinators::BoxBody;

        let headers = self.envelope().headers();

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
