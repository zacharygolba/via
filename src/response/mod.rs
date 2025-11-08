mod builder;
mod redirect;
mod response;

#[cfg(feature = "file")]
mod file;

pub(crate) mod body;

#[cfg(feature = "file")]
pub use file::File;

pub use body::{Json, ResponseBody};
pub use builder::{Finalize, ResponseBuilder};
pub use redirect::Redirect;
pub use response::Response;

pub(crate) use content_type::*;

pub(crate) mod content_type {
    use http::HeaderValue;

    pub const APPLICATION_JSON: HeaderValue =
        HeaderValue::from_static("application/json; charset=utf-8");

    pub const TEXT_HTML: HeaderValue = HeaderValue::from_static("text/html; charset=utf-8");
    pub const TEXT_PLAIN: HeaderValue = HeaderValue::from_static("text/plain; charset=utf-8");
}
