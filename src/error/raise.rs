/// Create a new error or decorate an existing one.
///
/// # Examples
///
/// Create a new error that uses the canonical reason prase of the provided
/// status code.
///
/// ```
/// via::err!(500);
/// ```
///
/// ### Decorate an existing error.
///
/// The generic impl of
/// [`From<E> for Error`](super::Error#impl-From<E>-for-Error)
/// uses `500` as the status code. It is often times desirable to provide a
/// more appropriate status code when the error is in context rather than using
/// dynamic typing to determine the status code that should be used in a
/// [`Rescue`](super::Rescue)
/// callback.
///
/// ```
/// use std::io;
/// use via::err;
///
/// fn invalid_input() -> io::Result<()> {
///     Err(io::ErrorKind::InvalidInput.into())
/// }
///
/// // Unboxed error types are passed as the second positional argument.
/// invalid_input().map_err(|error| err!(400, error));
///
/// // If the error source is already boxed, specify so to avoid allocating.
/// invalid_input().map_err(|error| err!(400, boxed = Box::new(error)));
/// ```
///
/// ### Customizing the error message.
///
/// The `err!` macro also allows you to provide a custom error message. The
/// message argument accepts `impl Into<String>`. Passing an owned `String` is
/// no less efficient than passing a `message = &'static str`.
///
/// ```
/// // Implicit allocation for message:
/// via::err!(404, message = "Could not find a user with the provided id.");
///
/// // Explicit allocation for message:
/// via::err!(404, message = format!("User with id: {} does not exist.", 12345));
/// ```
///
#[macro_export]
macro_rules! err {
    (message = $message:expr $(,)?) => { $crate::err!(500, message = $message) };
    (boxed = $source:expr $(,)?) => { $crate::err!(500, boxed = $source) };
    ($($args:tt)*) => { $crate::__via_impl_err!($($args)*) };
}

/// Wrap the output of `err!` in a result and return early.
///
/// # Example
///
/// ```
/// use http::header::AUTHORIZATION;
/// use via::{Next, Request, raise};
///
/// async fn authenticate(request: Request, next: Next) -> via::Result {
///     let Some(jwt) = request.header(AUTHORIZATION)? else {
///         raise!(401, message = "Missing required header: Authorization.")
///     };
///
///     // Insert JWT-based authentication strategy here.
///
///     next.call(request).await
/// }
/// ```
///
#[macro_export]
macro_rules! raise {
    ($($args:tt)*) => { return Err($crate::err!($($args)*)) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __via_impl_err {
    /*
     * Error constructor delegation
     */
    (@ctor $status:expr, message = $message:expr $(,)?) => {
        $crate::Error::new($status, $message)
    };
    (@ctor $status:expr, boxed = $source:expr $(,)?) => {
        $crate::Error::from_source($status, $source)
    };
    (@ctor $status:expr, $source:expr $(,)?) => {
        $crate::Error::from_source($status, Box::new($source))
    };
    (@ctor $status:expr) => {{
        let status = $status;
        let message = status.canonical_reason().unwrap_or_default().to_owned();
        $crate::Error::new(status, message)
    }};

    /*
     * Expand a status identifier to an expr using a fully-qualified path.
     */
    ($status:ident $($args:tt)*) => {
        $crate::__via_impl_err!(@ctor $crate::error::StatusCode::$status $($args)*)
    };

    /*
     * Define standard (or well-known) error status codes.
     */
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

    /*
     * Non-standard error status code support.
     */
    ($code:literal $($args:tt)*) => {{
        const CODE: u16 = $code;
        const _: () = assert!(
            CODE >= 400 && CODE <= 599,
            "Status code must be in 400..=599 for errors.",
        );

        let Ok(status) = $crate::error::StatusCode::from_u16(CODE) else {
            unreachable!()
        };

        $crate::__via_impl_err!(@ctor status $($args)*)
    }};
}
