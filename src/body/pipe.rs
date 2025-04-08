use bytes::Bytes;
use futures_core::Stream;
use http::header::TRANSFER_ENCODING;
use http_body::Frame;
use http_body_util::combinators::BoxBody;
use http_body_util::StreamBody;

use crate::error::{DynError, Error};
use crate::response::{Response, ResponseBuilder};

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
pub trait Pipe {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error>;
}

impl<T> Pipe for T
where
    T: Stream<Item = Result<Frame<Bytes>, DynError>> + Send + Sync + 'static,
{
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(BoxBody::new(StreamBody::new(self)))
    }
}
