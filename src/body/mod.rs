//! Asynchronously interact with [Request](crate::Request) and
//! [Response](crate::Response) bodies.

pub mod util;

mod any;
mod buffered;
mod frame_ext;

pub use any::AnyBody;
pub use buffered::BufferedBody;
pub use frame_ext::FrameExt;
