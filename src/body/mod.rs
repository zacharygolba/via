//! Asynchronously interact with [Request](crate::Request) and
//! [Response](crate::Response) bodies.
//!

pub mod util;

mod any;
mod boxed;
mod buffer;

pub use any::AnyBody;
pub use boxed::BoxBody;
pub use buffer::ByteBuffer;
