pub use hyper::body::Frame;

use super::Bytes;
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
        if self.is_trailers() {
            // We're only interested in data frames. Return early.
            return Ok(self);
        }

        // Unwrap the frame's data and map it using the provided closure.
        match f(self.into_data().unwrap()) {
            // The data was successfully mapped. Return `Ok` with a new frame
            // containing the mapped data.
            Ok(data) => Ok(Frame::data(data)),
            // An error occurred while mapping the data. Convert the error into
            // an `Error` and return.
            Err(error) => Err(error.into()),
        }
    }
}
