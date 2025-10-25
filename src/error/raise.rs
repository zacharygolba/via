/// Construct a new [`Error`](super::Error) or wrap an existing one by
/// providing a status code.
///
/// # Example
///
/// ```
/// use std::io;
/// use via::err;
///
/// // Use the canonical reason phrase of the status code as the message.
/// err!(404);
///
/// // Or provide a custom error message.
/// err!(404, message = "Could not find a user with the provided id.");
///
/// // You can allocate if you want.
/// err!(404, message = format!("User with id: {} does not exist.", 1234));
///
/// // Implicitly box the error source.
/// let io_error = io::Error::from(io::ErrorKind::InvalidInput);
/// let error = err!(400, io_error);
///
/// // Or specify when the error source is already boxed.
/// let io_error = io::Error::from(io::ErrorKind::InvalidInput);
/// let error = err!(400, boxed = Box::new(io_error));
/// ```
///
#[macro_export]
macro_rules! err {
    (400 $($arg:tt)*) => { $crate::err!(@ctor BAD_REQUEST $($arg)*) };
    (401 $($arg:tt)*) => { $crate::err!(@ctor UNAUTHORIZED $($arg)*) };
    (402 $($arg:tt)*) => { $crate::err!(@ctor PAYMENT_REQUIRED $($arg)*) };
    (403 $($arg:tt)*) => { $crate::err!(@ctor FORBIDDEN $($arg)*) };
    (404 $($arg:tt)*) => { $crate::err!(@ctor NOT_FOUND $($arg)*) };
    (405 $($arg:tt)*) => { $crate::err!(@ctor METHOD_NOT_ALLOWED $($arg)*) };
    (406 $($arg:tt)*) => { $crate::err!(@ctor NOT_ACCEPTABLE $($arg)*) };
    (407 $($arg:tt)*) => { $crate::err!(@ctor PROXY_AUTHENTICATION_REQUIRED $($arg)*) };
    (408 $($arg:tt)*) => { $crate::err!(@ctor REQUEST_TIMEOUT $($arg)*) };
    (409 $($arg:tt)*) => { $crate::err!(@ctor CONFLICT $($arg)*) };
    (410 $($arg:tt)*) => { $crate::err!(@ctor GONE $($arg)*) };
    (411 $($arg:tt)*) => { $crate::err!(@ctor LENGTH_REQUIRED $($arg)*) };
    (412 $($arg:tt)*) => { $crate::err!(@ctor PRECONDITION_FAILED $($arg)*) };
    (413 $($arg:tt)*) => { $crate::err!(@ctor PAYLOAD_TOO_LARGE $($arg)*) };
    (414 $($arg:tt)*) => { $crate::err!(@ctor URI_TOO_LONG $($arg)*) };
    (415 $($arg:tt)*) => { $crate::err!(@ctor UNSUPPORTED_MEDIA_TYPE $($arg)*) };
    (416 $($arg:tt)*) => { $crate::err!(@ctor RANGE_NOT_SATISFIABLE $($arg)*) };
    (417 $($arg:tt)*) => { $crate::err!(@ctor EXPECTATION_FAILED $($arg)*) };
    (418 $($arg:tt)*) => { $crate::err!(@ctor IM_A_TEAPOT $($arg)*) };
    (421 $($arg:tt)*) => { $crate::err!(@ctor MISDIRECTED_REQUEST $($arg)*) };
    (422 $($arg:tt)*) => { $crate::err!(@ctor UNPROCESSABLE_ENTITY $($arg)*) };
    (423 $($arg:tt)*) => { $crate::err!(@ctor LOCKED $($arg)*) };
    (424 $($arg:tt)*) => { $crate::err!(@ctor FAILED_DEPENDENCY $($arg)*) };
    (426 $($arg:tt)*) => { $crate::err!(@ctor UPGRADE_REQUIRED $($arg)*) };
    (428 $($arg:tt)*) => { $crate::err!(@ctor PRECONDITION_REQUIRED $($arg)*) };
    (429 $($arg:tt)*) => { $crate::err!(@ctor TOO_MANY_REQUESTS $($arg)*) };
    (431 $($arg:tt)*) => { $crate::err!(@ctor REQUEST_HEADER_FIELDS_TOO_LARGE $($arg)*) };
    (451 $($arg:tt)*) => { $crate::err!(@ctor UNAVAILABLE_FOR_LEGAL_REASONS $($arg)*) };
    (500 $($arg:tt)*) => { $crate::err!(@ctor INTERNAL_SERVER_ERROR $($arg)*) };
    (501 $($arg:tt)*) => { $crate::err!(@ctor NOT_IMPLEMENTED $($arg)*) };
    (502 $($arg:tt)*) => { $crate::err!(@ctor BAD_GATEWAY $($arg)*) };
    (503 $($arg:tt)*) => { $crate::err!(@ctor SERVICE_UNAVAILABLE $($arg)*) };
    (504 $($arg:tt)*) => { $crate::err!(@ctor GATEWAY_TIMEOUT $($arg)*) };
    (505 $($arg:tt)*) => { $crate::err!(@ctor HTTP_VERSION_NOT_SUPPORTED $($arg)*) };
    (506 $($arg:tt)*) => { $crate::err!(@ctor VARIANT_ALSO_NEGOTIATES $($arg)*) };
    (507 $($arg:tt)*) => { $crate::err!(@ctor INSUFFICIENT_STORAGE $($arg)*) };
    (508 $($arg:tt)*) => { $crate::err!(@ctor LOOP_DETECTED $($arg)*) };
    (510 $($arg:tt)*) => { $crate::err!(@ctor NOT_EXTENDED $($arg)*) };
    (511 $($arg:tt)*) => { $crate::err!(@ctor NETWORK_AUTHENTICATION_REQUIRED $($arg)*) };

    (message = $message:expr $(,)?) => { $crate::err!(500, message = $message) };
    (boxed = $source:expr $(,)?) => { $crate::err!(500, boxed = $source) };
    ($source:expr $(,)?) => { $crate::err!(500, $source) };

    (@ctor $status:ident, message = $message:expr $(,)?) => {
        $crate::Error::new($crate::error::StatusCode::$status, $message)
    };
    (@ctor $status:ident) => {{
        let status = $crate::error::StatusCode::$status;
        let message = status.canonical_reason().unwrap_or_default().to_owned();
        $crate::Error::new(status, message)
    }};
    (@ctor $status:ident, boxed = $source:expr $(,)?) => {
        $crate::Error::from_source($crate::error::StatusCode::$status, $source)
    };
    (@ctor $status:ident, $source:expr $(,)?) => {
        $crate::err!(@ctor $status, boxed = Box::new($source))
    };
}

/// Return early with a [`Result::Err`] by delegating to the [`err!`] macro.
///
/// # Example
///
/// ```
/// use http::header::AUTHORIZATION;
/// use via::{App, Next, Request, raise};
///
/// let mut app = App::new(());
///
/// app.middleware(async |request: Request, next: Next| {
///     let Some(_) = request.header(AUTHORIZATION)? else {
///         raise!(403, message = "Missing required header: Authorization.")
///     };
///
///     next.call(request).await
/// });
/// ```
///
#[macro_export]
macro_rules! raise {
    ( $($arg:tt)* ) => { return Err($crate::err!($($arg)*)) };
}
