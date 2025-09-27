/// Wrap existing errors or construct new ones by supplying a status code.
///
/// # Example
///
/// ```
/// use via::raise;
/// use std::io;
///
/// // Use the canonical reason phrase of the status code as the message.
/// raise!(404);
///
/// // Or provide a custom error message.
/// raise!(404, message = "Could not find a user with the provided id.");
///
/// // You can allocate if you want.
/// raise!(404, message = format!("User with id: {} does not exist.", 1234));
///
/// let result = Err(io::Error::from(io::ErrorKind::InvalidInput));
///
/// // Implicitly box the error source.
/// result.clone().map_err(|error| raise!(400, error));
///
/// // Or specify when the error source is already boxed.
/// result.clone().map_err(|error| raise!(400, boxed = Box::new(error)));
/// ```
///
#[macro_export]
macro_rules! raise {
    (@reason $status:ident, message = $message:expr $(,)*) => {{
        $crate::Error::new(http::StatusCode::$status, $message)
    }};

    (@reason $status:ident, boxed = $source:expr) => {{
        $crate::Error::from_source(http::StatusCode::$status, $source)
    }};

    (@reason $status:ident, $source:expr) => {{
        $crate::Error::from_source(http::StatusCode::$status, Box::new($source))
    }};

    (400) => { $crate::raise!(400, message = "Bad request.") };
    (400, $($arg:tt)*) => {
        $crate::raise!(@reason BAD_REQUEST, $($arg)*)
    };

    (401) => { $crate::raise!(401, message = "Unauthorized.") };
    (401, $($arg:tt)*) => {
        $crate::raise!(@reason UNAUTHORIZED, $($arg)*)
    };

    (402) => { $crate::raise!(402, message = "Payment required.") };
    (402, $($arg:tt)*) => {
        $crate::raise!(@reason PAYMENT_REQUIRED, $($arg)*)
    };

    (403) => { $crate::raise!(403, message = "Forbidden.") };
    (403, $($arg:tt)*) => {
        $crate::raise!(@reason FORBIDDEN, $($arg)*)
    };

    (404) => { $crate::raise!(404, message = "Not found.") };
    (404, $($arg:tt)*) => {
        $crate::raise!(@reason NOT_FOUND, $($arg)*)
    };

    (405) => { $crate::raise!(405, message = "Method not allowed.") };
    (405, $($arg:tt)*) => {
        $crate::raise!(@reason METHOD_NOT_ALLOWED, $($arg)*)
    };

    (406) => { $crate::raise!(406, message = "Not acceptable.") };
    (406, $($arg:tt)*) => {
        $crate::raise!(@reason NOT_ACCEPTABLE, $($arg)*)
    };

    (407) => { $crate::raise!(407, message = "Proxy authentication required.") };
    (407, $($arg:tt)*) => {
        $crate::raise!(@reason PROXY_AUTHENTICATION_REQUIRED, $($arg)*)
    };

    (408) => { $crate::raise!(408, message = "Request timeout.") };
    (408, $($arg:tt)*) => {
        $crate::raise!(@reason REQUEST_TIMEOUT, $($arg)*)
    };

    (409) => { $crate::raise!(409, message = "Conflict.") };
    (409, $($arg:tt)*) => {
        $crate::raise!(@reason CONFLICT, $($arg)*)
    };

    (410) => { $crate::raise!(410, message = "Gone.") };
    (410, $($arg:tt)*) => {
        $crate::raise!(@reason GONE, $($arg)*)
    };

    (411) => { $crate::raise!(411, message = "Length required.") };
    (411, $($arg:tt)*) => {
        $crate::raise!(@reason LENGTH_REQUIRED, $($arg)*)
    };

    (412) => { $crate::raise!(412, message = "Precondition failed.") };
    (412, $($arg:tt)*) => {
        $crate::raise!(@reason PRECONDITION_FAILED, $($arg)*)
    };

    (413) => { $crate::raise!(413, message = "Payload too large.") };
    (413, $($arg:tt)*) => {
        $crate::raise!(@reason PAYLOAD_TOO_LARGE, $($arg)*)
    };

    (414) => { $crate::raise!(414, message = "URI too long.") };
    (414, $($arg:tt)*) => {
        $crate::raise!(@reason URI_TOO_LONG, $($arg)*)
    };

    (415) => { $crate::raise!(415, message = "Unsupported media type.") };
    (415, $($arg:tt)*) => {
        $crate::raise!(@reason UNSUPPORTED_MEDIA_TYPE, $($arg)*)
    };

    (416) => { $crate::raise!(416, message = "Range not satisfiable.") };
    (416, $($arg:tt)*) => {
        $crate::raise!(@reason RANGE_NOT_SATISFIABLE, $($arg)*)
    };

    (417) => { $crate::raise!(417, message = "Expectation failed.") };
    (417, $($arg:tt)*) => {
        $crate::raise!(@reason EXPECTATION_FAILED, $($arg)*)
    };

    (418) => { $crate::raise!(418, message = "I'm a teapot.") };
    (418, $($arg:tt)*) => {
        $crate::raise!(@reason IM_A_TEAPOT, $($arg)*)
    };

    (421) => { $crate::raise!(421, message = "Misdirected request.") };
    (421, $($arg:tt)*) => {
        $crate::raise!(@reason MISDIRECTED_REQUEST, $($arg)*)
    };

    (422) => { $crate::raise!(422, message = "Unprocessable entity.") };
    (422, $($arg:tt)*) => {
        $crate::raise!(@reason UNPROCESSABLE_ENTITY, $($arg)*)
    };

    (423) => { $crate::raise!(423, message = "Locked.") };
    (423, $($arg:tt)*) => {
        $crate::raise!(@reason LOCKED, $($arg)*)
    };

    (424) => { $crate::raise!(424, message = "Failed dependency.") };
    (424, $($arg:tt)*) => {
        $crate::raise!(@reason FAILED_DEPENDENCY, $($arg)*)
    };

    (426) => { $crate::raise!(426, message = "Upgrade required.") };
    (426, $($arg:tt)*) => {
        $crate::raise!(@reason UPGRADE_REQUIRED, $($arg)*)
    };

    (428) => { $crate::raise!(428, message = "Precondition required.") };
    (428, $($arg:tt)*) => {
        $crate::raise!(@reason PRECONDITION_REQUIRED, $($arg)*)
    };

    (429) => { $crate::raise!(429, message = "Too many requests.") };
    (429, $($arg:tt)*) => {
        $crate::raise!(@reason TOO_MANY_REQUESTS, $($arg)*)
    };

    (431) => { $crate::raise!(431, message = "Request header fields too large.") };
    (431, $($arg:tt)*) => {
        $crate::raise!(@reason REQUEST_HEADER_FIELDS_TOO_LARGE, $($arg)*)
    };

    (451) => { $crate::raise!(451, message = "Unavailable for legal reasons.") };
    (451, $($arg:tt)*) => {
        $crate::raise!(@reason UNAVAILABLE_FOR_LEGAL_REASONS, $($arg)*)
    };

    (500) => { $crate::raise!(500, message = "Internal server error.") };
    (500, $($arg:tt)*) => {
        $crate::raise!(@reason INTERNAL_SERVER_ERROR, $($arg)*)
    };

    (501) => { $crate::raise!(501, message = "Not implemented.") };
    (501, $($arg:tt)*) => {
        $crate::raise!(@reason NOT_IMPLEMENTED, $($arg)*)
    };

    (502) => { $crate::raise!(502, message = "Bad gateway.") };
    (502, $($arg:tt)*) => {
        $crate::raise!(@reason BAD_GATEWAY, $($arg)*)
    };

    (503) => { $crate::raise!(503, message = "Service unavailable.") };
    (503, $($arg:tt)*) => {
        $crate::raise!(@reason SERVICE_UNAVAILABLE, $($arg)*)
    };

    (504) => { $crate::raise!(504, message = "Gateway timeout.") };
    (504, $($arg:tt)*) => {
        $crate::raise!(@reason GATEWAY_TIMEOUT, $($arg)*)
    };

    (505) => { $crate::raise!(505, message = "HTTP version not supported.") };
    (505, $($arg:tt)*) => {
        $crate::raise!(@reason HTTP_VERSION_NOT_SUPPORTED, $($arg)*)
    };

    (506) => { $crate::raise!(506, message = "Variant also negotiates.") };
    (506, $($arg:tt)*) => {
        $crate::raise!(@reason VARIANT_ALSO_NEGOTIATES, $($arg)*)
    };

    (507) => { $crate::raise!(507, message = "Insufficient storage.") };
    (507, $($arg:tt)*) => {
        $crate::raise!(@reason INSUFFICIENT_STORAGE, $($arg)*)
    };

    (508) => { $crate::raise!(508, message = "Loop detected.") };
    (508, $($arg:tt)*) => {
        $crate::raise!(@reason LOOP_DETECTED, $($arg)*)
    };

    (510) => { $crate::raise!(510, message = "Not extended.") };
    (510, $($arg:tt)*) => {
        $crate::raise!(@reason NOT_EXTENDED, $($arg)*)
    };

    (511) => { $crate::raise!(511, message = "Network authentication required.") };
    (511, $($arg:tt)*) => {
        $crate::raise!(@reason NETWORK_AUTHENTICATION_REQUIRED, $($arg)*)
    };
}
