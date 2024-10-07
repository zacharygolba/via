//! Asynchronously interact with [Request](crate::Request) and
//! [Response](crate::Response) bodies.
//!

pub mod size_hint;

mod any;
mod buffer;

pub use any::AnyBody;
pub use buffer::ByteBuffer;
