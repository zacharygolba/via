use bytes::Bytes;

pub use hyper::body::Frame;

use crate::Error;

/// An extension trait for [`Frame`](super::Frame) that includes convenience
/// methods for common operations.
pub trait FrameExt {
    /// Attempts to map the frame's data using the provided closure.
    fn try_map_data<F, E>(self, f: F) -> Result<Frame<Bytes>, Error>
    where
        F: FnOnce(Bytes) -> Result<Bytes, E>,
        E: Into<Error>;
}

impl FrameExt for Frame<Bytes> {
    fn try_map_data<F, E>(self, f: F) -> Result<Frame<Bytes>, Error>
    where
        F: FnOnce(Bytes) -> Result<Bytes, E>,
        E: Into<Error>,
    {
        match self.into_data() {
            // The frame contains data. Apply our map fn and return the result.
            Ok(data) => f(data).map(Frame::data).map_err(|error| error.into()),
            // The frame contains trailers. Do not apply the map fn.
            Err(frame) => Ok(frame),
        }
    }
}
