pub mod aggregate;
pub mod stream;
pub mod util;

mod any;
mod boxed;
mod buffered;
mod frame_ext;
mod stream_adapter;

/// A re-export of
/// [`hyper::body::Body`](https://docs.rs/hyper/latest/hyper/body/trait.Body.html).
pub use hyper::body::Body;

/// A re-export of
/// [`hyper::body::Frame`](https://docs.rs/hyper/latest/hyper/body/struct.Frame.html).
pub use hyper::body::Frame;

/// A re-export of
/// [`hyper::body::Incoming`](https://docs.rs/hyper/latest/hyper/body/struct.Incoming.html).
pub use hyper::body::Incoming;

pub use any::AnyBody;
pub use boxed::Boxed;
pub use buffered::Buffered;
pub use frame_ext::FrameExt;

pub(crate) use stream_adapter::StreamAdapter;
