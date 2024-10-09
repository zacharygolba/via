mod body;
mod builder;
mod into_response;
mod redirect;
mod response;
mod stream_adapter;

pub use body::ResponseBody;
pub use builder::ResponseBuilder;
pub use into_response::IntoResponse;
pub use redirect::Redirect;
pub use response::Response;

pub(crate) use response::OutgoingResponse;

use http::HeaderValue;

use stream_adapter::StreamAdapter;

const APPLICATION_JSON: HeaderValue = HeaderValue::from_static("application/json; charset=utf-8");
const CHUNKED_ENCODING: HeaderValue = HeaderValue::from_static("chunked");
const TEXT_PLAIN: HeaderValue = HeaderValue::from_static("text/plain; charset=utf-8");
const TEXT_HTML: HeaderValue = HeaderValue::from_static("text/html; charset=utf-8");
