#![allow(private_bounds)]

use bytes::Bytes;
use futures_core::Stream;
use http::header::TRANSFER_ENCODING;

use super::stream_body::StreamBody;
use crate::body::{BoxBody, HttpBody, RequestBody};
use crate::error::{DynError, Error};
use crate::response::{Response, ResponseBuilder};

trait Sealed {}

/// Pipe frames from a [`Stream`]-like type to a response body.
///
/// ```
/// use http::header::CONTENT_TYPE;
/// use via::{Next, Request, Response, Pipe};
///
/// async fn echo(request: Request, _: Next) -> via::Result {
///     let mut response = Response::build();
///
///     if let Some(content_type) = request.header(CONTENT_TYPE).cloned() {
///         response = response.header(CONTENT_TYPE, content_type);
///     }
///
///     request.into_body().pipe(response)
/// }
/// ```
///
pub trait Pipe: Sealed {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error>;
}

impl Sealed for HttpBody<RequestBody> {}

impl Pipe for HttpBody<RequestBody> {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Mapped(self.boxed()))
    }
}

impl<T> Sealed for T where T: Stream<Item = Result<Bytes, DynError>> + Send + Sync + 'static {}

impl<T> Pipe for T
where
    T: Stream<Item = Result<Bytes, DynError>> + Send + Sync + 'static,
{
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Mapped(BoxBody::new(StreamBody::new(self))))
    }
}
