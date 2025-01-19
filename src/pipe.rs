use bytes::Bytes;
use futures_core::Stream;
use http::header::TRANSFER_ENCODING;
use http_body::Frame;

use crate::body::{BoxBody, HttpBody, RequestBody, StreamBody};
use crate::error::{BoxError, Error};
use crate::response::{Builder, Response};

/// Defines how a [`Stream`]-like type can be used as a response body.
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
pub trait Pipe {
    fn pipe(self, response: Builder) -> Result<Response, Error>;
}

impl Pipe for HttpBody<RequestBody> {
    fn pipe(self, response: Builder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Mapped(match self {
                HttpBody::Original(body) => BoxBody::new(body),
                HttpBody::Mapped(body) => body,
            }))
    }
}

impl<T> Pipe for T
where
    T: Stream<Item = Result<Frame<Bytes>, BoxError>> + Send + Sync + 'static,
{
    fn pipe(self, response: Builder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Mapped(BoxBody::new(StreamBody::new(self))))
    }
}
