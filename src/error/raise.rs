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
    (message = $message:expr $(,)?) => { $crate::err!(500, message = $message) };
    (boxed = $source:expr $(,)?) => { $crate::err!(500, boxed = $source) };
    ($($args:tt)*) => { $crate::__via_impl_err!($($args)*) };
}

/// Return early with a [`Result::Err`] by delegating to the [`err!`] macro.
///
/// # Example
///
/// ```
/// use http::header::AUTHORIZATION;
/// use via::{App, Error, Next, Request, raise};
///
/// let mut app = App::new(());
///
/// app.middleware(async |request: Request, next: Next| {
///     let Some(jwt) = request.header(AUTHORIZATION)? else {
///         raise!(401, message = "Missing required header: Authorization.")
///     };
///
///     // Insert JWT-based authentication code here.
///
///     next.call(request).await
/// });
/// ```
///
#[macro_export]
macro_rules! raise {
    ($($args:tt)*) => { return Err($crate::err!($($args)*)) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __via_impl_err {
    (400 $($args:tt)*) => { $crate::__via_impl_err!(BAD_REQUEST $($args)*) };
    (401 $($args:tt)*) => { $crate::__via_impl_err!(UNAUTHORIZED $($args)*) };
    (402 $($args:tt)*) => { $crate::__via_impl_err!(PAYMENT_REQUIRED $($args)*) };
    (403 $($args:tt)*) => { $crate::__via_impl_err!(FORBIDDEN $($args)*) };
    (404 $($args:tt)*) => { $crate::__via_impl_err!(NOT_FOUND $($args)*) };
    (405 $($args:tt)*) => { $crate::__via_impl_err!(METHOD_NOT_ALLOWED $($args)*) };
    (406 $($args:tt)*) => { $crate::__via_impl_err!(NOT_ACCEPTABLE $($args)*) };
    (407 $($args:tt)*) => { $crate::__via_impl_err!(PROXY_AUTHENTICATION_REQUIRED $($args)*) };
    (408 $($args:tt)*) => { $crate::__via_impl_err!(REQUEST_TIMEOUT $($args)*) };
    (409 $($args:tt)*) => { $crate::__via_impl_err!(CONFLICT $($args)*) };
    (410 $($args:tt)*) => { $crate::__via_impl_err!(GONE $($args)*) };
    (411 $($args:tt)*) => { $crate::__via_impl_err!(LENGTH_REQUIRED $($args)*) };
    (412 $($args:tt)*) => { $crate::__via_impl_err!(PRECONDITION_FAILED $($args)*) };
    (413 $($args:tt)*) => { $crate::__via_impl_err!(PAYLOAD_TOO_LARGE $($args)*) };
    (414 $($args:tt)*) => { $crate::__via_impl_err!(URI_TOO_LONG $($args)*) };
    (415 $($args:tt)*) => { $crate::__via_impl_err!(UNSUPPORTED_MEDIA_TYPE $($args)*) };
    (416 $($args:tt)*) => { $crate::__via_impl_err!(RANGE_NOT_SATISFIABLE $($args)*) };
    (417 $($args:tt)*) => { $crate::__via_impl_err!(EXPECTATION_FAILED $($args)*) };
    (418 $($args:tt)*) => { $crate::__via_impl_err!(IM_A_TEAPOT $($args)*) };
    (421 $($args:tt)*) => { $crate::__via_impl_err!(MISDIRECTED_REQUEST $($args)*) };
    (422 $($args:tt)*) => { $crate::__via_impl_err!(UNPROCESSABLE_ENTITY $($args)*) };
    (423 $($args:tt)*) => { $crate::__via_impl_err!(LOCKED $($args)*) };
    (424 $($args:tt)*) => { $crate::__via_impl_err!(FAILED_DEPENDENCY $($args)*) };
    (426 $($args:tt)*) => { $crate::__via_impl_err!(UPGRADE_REQUIRED $($args)*) };
    (428 $($args:tt)*) => { $crate::__via_impl_err!(PRECONDITION_REQUIRED $($args)*) };
    (429 $($args:tt)*) => { $crate::__via_impl_err!(TOO_MANY_REQUESTS $($args)*) };
    (431 $($args:tt)*) => { $crate::__via_impl_err!(REQUEST_HEADER_FIELDS_TOO_LARGE $($args)*) };
    (451 $($args:tt)*) => { $crate::__via_impl_err!(UNAVAILABLE_FOR_LEGAL_REASONS $($args)*) };
    (500 $($args:tt)*) => { $crate::__via_impl_err!(INTERNAL_SERVER_ERROR $($args)*) };
    (501 $($args:tt)*) => { $crate::__via_impl_err!(NOT_IMPLEMENTED $($args)*) };
    (502 $($args:tt)*) => { $crate::__via_impl_err!(BAD_GATEWAY $($args)*) };
    (503 $($args:tt)*) => { $crate::__via_impl_err!(SERVICE_UNAVAILABLE $($args)*) };
    (504 $($args:tt)*) => { $crate::__via_impl_err!(GATEWAY_TIMEOUT $($args)*) };
    (505 $($args:tt)*) => { $crate::__via_impl_err!(HTTP_VERSION_NOT_SUPPORTED $($args)*) };
    (506 $($args:tt)*) => { $crate::__via_impl_err!(VARIANT_ALSO_NEGOTIATES $($args)*) };
    (507 $($args:tt)*) => { $crate::__via_impl_err!(INSUFFICIENT_STORAGE $($args)*) };
    (508 $($args:tt)*) => { $crate::__via_impl_err!(LOOP_DETECTED $($args)*) };
    (510 $($args:tt)*) => { $crate::__via_impl_err!(NOT_EXTENDED $($args)*) };
    (511 $($args:tt)*) => { $crate::__via_impl_err!(NETWORK_AUTHENTICATION_REQUIRED $($args)*) };

    ($status:ident, message = $message:expr $(,)?) => {
        $crate::Error::new($crate::error::StatusCode::$status, $message)
    };
    ($status:ident, boxed = $source:expr $(,)?) => {
        $crate::Error::from_source($crate::error::StatusCode::$status, $source)
    };
    ($status:ident, $source:expr $(,)?) => {
        $crate::Error::from_source($crate::error::StatusCode::$status, Box::new($source))
    };
    ($status:ident) => {{
        let status = $crate::error::StatusCode::$status;
        let message = status.canonical_reason().unwrap_or_default().to_owned();
        $crate::Error::new(status, message)
    }};
}
