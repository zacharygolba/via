#![allow(private_bounds)]

use bytes::Bytes;
use futures_core::Stream;
use http::header::TRANSFER_ENCODING;
use http_body::Frame;

use super::stream_body::StreamBody;
use crate::body::{BoxBody, HttpBody, RequestBody};
use crate::error::{DynError, Error};
use crate::response::{Builder, Response};

trait Sealed {}

/// Pipe frames from a [`Stream`]-like type to a response body.
///
/// ```
/// use http::header::CONTENT_TYPE;
/// use via::{Next, Request, Response, Pipe};
///
/// async fn echo(request: Request, _: Next) -> via::Result {
///     let content_type = request.header(CONTENT_TYPE).cloned();
///     let response = Response::build().headers([(CONTENT_TYPE, content_type)]);
///
///     request.into_body().pipe(response)
/// }
/// ```
///
pub trait Pipe: Sealed {
    fn pipe(self, response: Builder) -> Result<Response, Error>;
}

impl Sealed for HttpBody<RequestBody> {}

impl Pipe for HttpBody<RequestBody> {
    fn pipe(self, response: Builder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Mapped(self.boxed()))
    }
}

impl<T> Sealed for T where T: Stream<Item = Result<Frame<Bytes>, DynError>> + Send + Sync + 'static {}

impl<T> Pipe for T
where
    T: Stream<Item = Result<Frame<Bytes>, DynError>> + Send + Sync + 'static,
{
    fn pipe(self, response: Builder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Mapped(BoxBody::new(StreamBody::new(self))))
    }
}
