use bytes::Bytes;
use futures_core::Stream;
use http::header::TRANSFER_ENCODING;
use http_body::Frame;
use http_body_util::StreamBody;
use http_body_util::combinators::BoxBody;

use crate::error::{BoxError, Error};
use crate::response::{Response, ResponseBuilder};

/// Define how a type becomes a [`Response`].
///
/// ```
/// use via::{Next, Request, Response, Pipe};
///
/// async fn echo(request: Request, _: Next) -> via::Result {
///     let mut response = Response::build();
///     request.pipe(response.header("X-Powered-By", "Via"))
/// }
/// ```
///
pub trait Pipe {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error>;
}

impl<T> Pipe for T
where
    T: Stream<Item = Result<Frame<Bytes>, BoxError>> + Send + Sync + 'static,
{
    fn pipe(self, builder: ResponseBuilder) -> Result<Response, Error> {
        builder
            .header(TRANSFER_ENCODING, "chunked")
            .body(BoxBody::new(StreamBody::new(self)))
    }
}
