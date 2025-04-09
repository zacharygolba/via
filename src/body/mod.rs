mod body_reader;
mod body_stream;
mod buffer_body;
mod pipe;
mod util;

pub use body_reader::{BodyData, BodyReader};
pub use body_stream::BodyStream;
pub use buffer_body::BufferBody;
pub use pipe::Pipe;

#[allow(unused_imports)]
pub(crate) use buffer_body::MAX_FRAME_LEN;

/// A type erased, dynamically dispatched [`Body`](http_body::Body).
///
pub type BoxBody = http_body_util::combinators::BoxBody<bytes::Bytes, crate::error::DynError>;
