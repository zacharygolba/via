//! Conviently work with errors that may occur in an application.
//!

use http::StatusCode;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};
use std::io;
use tokio::task::JoinError;

use crate::response::{Response, ResponseBody};

/// A type alias for a boxed `dyn Error + Send + Sync`.
///
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// An error type that can act as a specialized version of a
/// [`ResponseBuilder`](crate::response::ResponseBuilder).
///
#[derive(Debug)]
pub struct Error {
    as_json: bool,
    status: StatusCode,
    error: BoxError,
    fmt: Option<String>,
}

#[doc(hidden)]
#[derive(Debug, Serialize)]
pub struct Message<'a> {
    message: Cow<'a, str>,
}

#[derive(Debug)]
pub(crate) enum ServerError {
    Io(io::Error),
    Join(JoinError),
    Hyper(hyper::Error),
}

#[macro_export]
macro_rules! error {
    (@reason $ctor:ident, $($arg:tt)*) => {{
        use $crate::error::{Error, Message};
        Error::$ctor(Message::new(format!($($arg)*)))
    }};

    (400) => { $crate::error!(400, "Bad request.") };
    (400, $($arg:tt)*) => {
        $crate::error!(@reason bad_request, $($arg)*)
    };

    (401) => { $crate::error!(401, "Unauthorized.") };
    (401, $($arg:tt)*) => {
        $crate::error!(@reason unauthorized, $($arg)*)
    };

    (402) => { $crate::error!(402, "Payment required.") };
    (402, $($arg:tt)*) => {
        $crate::error!(@reason payment_required, $($arg)*)
    };

    (403) => { $crate::error!(403, "Forbidden.") };
    (403, $($arg:tt)*) => {
        $crate::error!(@reason forbidden, $($arg)*)
    };

    (404) => { $crate::error!(404, "Not found.") };
    (404, $($arg:tt)*) => {
        $crate::error!(@reason not_found, $($arg)*)
    };

    (405) => { $crate::error!(405, "Method not allowed.") };
    (405, $($arg:tt)*) => {
        $crate::error!(@reason method_not_allowed, $($arg)*)
    };

    (406) => { $crate::error!(406, "Not acceptable.") };
    (406, $($arg:tt)*) => {
        $crate::error!(@reason not_acceptable, $($arg)*)
    };

    (407) => { $crate::error!(407, "Proxy authentication required.") };
    (407, $($arg:tt)*) => {
        $crate::error!(@reason proxy_authentication_required, $($arg)*)
    };

    (408) => { $crate::error!(408, "Request timeout.") };
    (408, $($arg:tt)*) => {
        $crate::error!(@reason request_timeout, $($arg)*)
    };

    (409) => { $crate::error!(409, "Conflict.") };
    (409, $($arg:tt)*) => {
        $crate::error!(@reason conflict, $($arg)*)
    };

    (410) => { $crate::error!(410, "Gone.") };
    (410, $($arg:tt)*) => {
        $crate::error!(@reason gone, $($arg)*)
    };

    (411) => { $crate::error!(411, "Length required.") };
    (411, $($arg:tt)*) => {
        $crate::error!(@reason length_required, $($arg)*)
    };

    (412) => { $crate::error!(412, "Precondition failed.") };
    (412, $($arg:tt)*) => {
        $crate::error!(@reason precondition_failed, $($arg)*)
    };

    (413) => { $crate::error!(413, "Payload too large.") };
    (413, $($arg:tt)*) => {
        $crate::error!(@reason payload_too_large, $($arg)*)
    };

    (414) => { $crate::error!(414, "URI too long.") };
    (414, $($arg:tt)*) => {
        $crate::error!(@reason uri_too_long, $($arg)*)
    };

    (415) => { $crate::error!(415, "Unsupported media type.") };
    (415, $($arg:tt)*) => {
        $crate::error!(@reason unsupported_media_type, $($arg)*)
    };

    (416) => { $crate::error!(416, "Range not satisfiable.") };
    (416, $($arg:tt)*) => {
        $crate::error!(@reason range_not_satisfiable, $($arg)*)
    };

    (417) => { $crate::error!(417, "Expectation failed.") };
    (417, $($arg:tt)*) => {
        $crate::error!(@reason expectation_failed, $($arg)*)
    };

    (418) => { $crate::error!(418, "I'm a teapot.") };
    (418, $($arg:tt)*) => {
        $crate::error!(@reason im_a_teapot, $($arg)*)
    };

    (421) => { $crate::error!(421, "Misdirected request.") };
    (421, $($arg:tt)*) => {
        $crate::error!(@reason misdirected_request, $($arg)*)
    };

    (422) => { $crate::error!(422, "Unprocessable entity.") };
    (422, $($arg:tt)*) => {
        $crate::error!(@reason unprocessable_entity, $($arg)*)
    };

    (423) => { $crate::error!(423, "Locked.") };
    (423, $($arg:tt)*) => {
        $crate::error!(@reason locked, $($arg)*)
    };

    (424) => { $crate::error!(424, "Failed dependency.") };
    (424, $($arg:tt)*) => {
        $crate::error!(@reason failed_dependency, $($arg)*)
    };

    (426) => { $crate::error!(426, "Upgrade required.") };
    (426, $($arg:tt)*) => {
        $crate::error!(@reason upgrade_required, $($arg)*)
    };

    (428) => { $crate::error!(428, "Precondition required.") };
    (428, $($arg:tt)*) => {
        $crate::error!(@reason precondition_required, $($arg)*)
    };

    (429) => { $crate::error!(429, "Too many requests.") };
    (429, $($arg:tt)*) => {
        $crate::error!(@reason too_many_requests, $($arg)*)
    };

    (431) => { $crate::error!(431, "Request header fields too large.") };
    (431, $($arg:tt)*) => {
        $crate::error!(@reason request_header_fields_too_large, $($arg)*)
    };

    (451) => { $crate::error!(451, "Unavailable for legal reasons.") };
    (451, $($arg:tt)*) => {
        $crate::error!(@reason unavailable_for_legal_reasons, $($arg)*)
    };

    (500) => { $crate::error!(500, "Internal server error.") };
    (500, $($arg:tt)*) => {
        $crate::error!(@reason internal_server_error, $($arg)*)
    };

    (501) => { $crate::error!(501, "Not implemented.") };
    (501, $($arg:tt)*) => {
        $crate::error!(@reason not_implemented, $($arg)*)
    };

    (502) => { $crate::error!(502, "Bad gateway.") };
    (502, $($arg:tt)*) => {
        $crate::error!(@reason bad_gateway, $($arg)*)
    };

    (503) => { $crate::error!(503, "Service unavailable.") };
    (503, $($arg:tt)*) => {
        $crate::error!(@reason service_unavailable, $($arg)*)
    };

    (504) => { $crate::error!(504, "Gateway timeout.") };
    (504, $($arg:tt)*) => {
        $crate::error!(@reason gateway_timeout, $($arg)*)
    };

    (505) => { $crate::error!(505, "HTTP version not supported.") };
    (505, $($arg:tt)*) => {
        $crate::error!(@reason http_version_not_supported, $($arg)*)
    };

    (506) => { $crate::error!(506, "Variant also negotiates.") };
    (506, $($arg:tt)*) => {
        $crate::error!(@reason variant_also_negotiates, $($arg)*)
    };

    (507) => { $crate::error!(507, "Insufficient storage.") };
    (507, $($arg:tt)*) => {
        $crate::error!(@reason insufficient_storage, $($arg)*)
    };

    (508) => { $crate::error!(508, "Loop detected.") };
    (508, $($arg:tt)*) => {
        $crate::error!(@reason loop_detected, $($arg)*)
    };

    (510) => { $crate::error!(510, "Not extended.") };
    (510, $($arg:tt)*) => {
        $crate::error!(@reason not_extended, $($arg)*)
    };

    (511) => { $crate::error!(511, "Network authentication required.") };
    (511, $($arg:tt)*) => {
        $crate::error!(@reason network_authentication_required, $($arg)*)
    };
}

impl Error {
    /// Create a new [`Error`] from the provided status code and source.
    ///
    pub fn new(status: StatusCode, error: BoxError) -> Self {
        Self {
            as_json: false,
            status,
            error,
            fmt: None,
        }
    }

    /// Create a new [`Error`] from the provided [`io::Error`]. The status code
    /// of the error returned will correspond to `source.kind()`.
    ///
    pub fn from_io(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::AlreadyExists => {
                // Implies a resource already exists.
                Self::conflict(error)
            }

            io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted => {
                // Signals a broken connection.
                Self::bad_gateway(error)
            }

            io::ErrorKind::ConnectionRefused => {
                // Suggests the service is not ready or available.
                Self::service_unavailable(error)
            }

            io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput => {
                // Generally indicates a malformed request.
                Self::bad_request(error)
            }

            io::ErrorKind::IsADirectory
            | io::ErrorKind::NotADirectory
            | io::ErrorKind::PermissionDenied => {
                // Implies restricted access.
                Self::forbidden(error)
            }

            io::ErrorKind::NotFound => {
                // Indicates a missing resource.
                Self::not_found(error)
            }

            io::ErrorKind::TimedOut => {
                // Implies an upstream service timeout.
                Self::gateway_timeout(error)
            }

            _ => {
                // Any other kind is treated as an internal server error.
                Self::internal_server_error(error)
            }
        }
    }

    /// Returns a new [`Error`] that will be serialized to JSON when converted to
    /// a [`Response`].
    ///
    pub fn as_json(self) -> Self {
        Self {
            as_json: true,
            ..self
        }
    }

    /// Returns a new [`Error`] that will eagerly map the message that will be
    /// included in the body of the [`Response`] that will be generated from
    /// self by calling the provided closure. If the closure returns `None`,
    /// the message will be left unchanged.
    ///
    /// # Example
    ///
    /// ```
    /// use via::builtin::rescue;
    /// use via::{BoxError, Next, Request};
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<(), BoxError> {
    ///     let mut app = via::app(());
    ///
    ///     // Add a rescue middleware to the route tree that maps errors that
    ///     // occur in subsequent middleware by calling the `redact` function.
    ///     app.include(rescue::map(|error| {
    ///         error.redact(|message| {
    ///             if message.contains("password") {
    ///                 // If password is even mentioned in the error, return an
    ///                 // opaque message instead. You'll probably want something
    ///                 // more sophisticated than this in production.
    ///                 Some("An error occurred...".to_owned())
    ///             } else {
    ///                 // Otherwise, use the existing error message.
    ///                 None
    ///             }
    ///         })
    ///     }));
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub fn redact(self, f: impl FnOnce(&str) -> Option<String>) -> Self {
        match &self.fmt {
            Some(message) => match f(message) {
                Some(redacted) => self.with_message(redacted),
                None => self,
            },
            None => {
                let message = self.error.to_string();
                let redacted = f(&message).unwrap_or(message);

                self.with_message(redacted)
            }
        }
    }

    /// Returns a new [`Error`] that will use the provided message instead of
    /// calling the [`Display`] implementation of the error source when
    /// converted to a [`Response`].
    ///
    pub fn with_message(self, message: String) -> Self {
        Self {
            fmt: Some(message),
            ..self
        }
    }

    /// Sets the status code of that will be used when converted to a
    /// [`Response`].
    ///
    pub fn with_status(self, status: StatusCode) -> Self {
        // Placeholder for tracing...
        // Warn if the status code is not in the 4xx or 5xx range.
        Self { status, ..self }
    }

    /// Returns a new [`Error`] that will use the canonical reason phrase of the
    /// status code as the message included in the [`Response`] body that is
    /// generated when converted to a [`Response`].
    ///
    /// # Example
    ///
    /// ```
    /// use via::builtin::rescue;
    /// use via::{BoxError, Next, Request};
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<(), BoxError> {
    ///     let mut app = via::app(());
    ///
    ///     // Add a rescue middleware to the route tree that maps errors that
    ///     // occur in subsequent middleware by calling the
    ///     // `use_canonical_reason` function.
    ///     app.include(rescue::map(|error| {
    ///         // Log the original error so no context is lost.
    ///         eprintln!("error: {}", error);
    ///
    ///         // Prevent error messages that occur in downstream middleware
    ///         // from leaking into the response body by using the reason
    ///         // phrase of the status code associated with the error.
    ///         error.use_canonical_reason()
    ///     }));
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub fn use_canonical_reason(self) -> Self {
        if let Some(reason) = self.status.canonical_reason() {
            self.with_message(reason.to_owned())
        } else {
            // Placeholder for tracing...
            self.with_message("An error occurred".to_owned())
        }
    }

    /// Returns an iterator over the sources of this error.
    ///
    pub fn iter(&self) -> impl Iterator<Item = &dyn StdError> {
        Some(self.source()).into_iter()
    }

    /// Returns a reference to the error source.
    ///
    pub fn source(&self) -> &(dyn StdError + 'static) {
        &*self.error
    }
}

impl Error {
    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `400 Bad Request` status.
    ///
    pub fn bad_request<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::BAD_REQUEST, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `401 Unauthorized` status.
    ///
    pub fn unauthorized<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::UNAUTHORIZED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `402 Payment Required` status.
    ///
    pub fn payment_required<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::PAYMENT_REQUIRED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `403 Forbidden` status.
    ///
    pub fn forbidden<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::FORBIDDEN, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `404 Not Found` status.
    ///
    pub fn not_found<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::NOT_FOUND, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `405 Method Not Allowed` status.
    ///
    pub fn method_not_allowed<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::METHOD_NOT_ALLOWED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `406 Not Acceptable` status.
    ///
    pub fn not_acceptable<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::NOT_ACCEPTABLE, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `407 Proxy Authentication Required` status.
    ///
    pub fn proxy_authentication_required<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::PROXY_AUTHENTICATION_REQUIRED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `408 Request Timeout` status.
    ///
    pub fn request_timeout<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::REQUEST_TIMEOUT, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `409 Conflict` status.
    ///
    pub fn conflict<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::CONFLICT, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `410 Gone` status.
    ///
    pub fn gone<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::GONE, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `411 Length Required` status.
    ///
    pub fn length_required<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::LENGTH_REQUIRED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `412 Precondition Failed` status.
    ///
    pub fn precondition_failed<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::PRECONDITION_FAILED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `413 Payload Too Large` status.
    ///
    pub fn payload_too_large<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `414 URI Too Long` status.
    ///
    pub fn uri_too_long<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::URI_TOO_LONG, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `415 Unsupported Media Type` status.
    ///
    pub fn unsupported_media_type<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::UNSUPPORTED_MEDIA_TYPE, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `416 Range Not Satisfiable` status.
    ///
    pub fn range_not_satisfiable<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::RANGE_NOT_SATISFIABLE, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `417 Expectation Failed` status.
    ///
    pub fn expectation_failed<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::EXPECTATION_FAILED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `418 I'm a teapot` status.
    ///
    pub fn im_a_teapot<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::IM_A_TEAPOT, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `421 Misdirected Request` status.
    ///
    pub fn misdirected_request<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::MISDIRECTED_REQUEST, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `422 Unprocessable Entity` status.
    ///
    pub fn unprocessable_entity<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `423 Locked` status.
    ///
    pub fn locked<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::LOCKED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `424 Failed Dependency` status.
    ///
    pub fn failed_dependency<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::FAILED_DEPENDENCY, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `426 Upgrade Required` status.
    ///
    pub fn upgrade_required<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::UPGRADE_REQUIRED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `428 Precondition Required` status.
    ///
    pub fn precondition_required<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::PRECONDITION_REQUIRED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `429 Too Many Requests` status.
    ///
    pub fn too_many_requests<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::TOO_MANY_REQUESTS, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `431 Request Header Fields Too Large` status.
    ///
    pub fn request_header_fields_too_large<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            Box::new(source),
        )
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `451 Unavailable For Legal Reasons` status.
    ///
    pub fn unavailable_for_legal_reasons<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `500 Internal Server Error` status.
    ///
    pub fn internal_server_error<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `501 Not Implemented` status.
    ///
    pub fn not_implemented<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::NOT_IMPLEMENTED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `502 Bad Gateway` status.
    ///
    pub fn bad_gateway<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::BAD_GATEWAY, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `503 Service Unavailable` status.
    ///
    pub fn service_unavailable<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `504 Gateway Timeout` status.
    ///
    pub fn gateway_timeout<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::GATEWAY_TIMEOUT, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `505 HTTP Version Not Supported` status.
    ///
    pub fn http_version_not_supported<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::HTTP_VERSION_NOT_SUPPORTED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `506 Variant Also Negotiates` status.
    ///
    pub fn variant_also_negotiates<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::VARIANT_ALSO_NEGOTIATES, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `507 Insufficient Storage` status.
    ///
    pub fn insufficient_storage<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::INSUFFICIENT_STORAGE, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `508 Loop Detected` status.
    ///
    pub fn loop_detected<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::LOOP_DETECTED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `510 Not Extended` status.
    ///
    pub fn not_extended<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(StatusCode::NOT_EXTENDED, Box::new(source))
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `511 Network Authentication Required` status.
    ///
    pub fn network_authentication_required<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::new(
            StatusCode::NETWORK_AUTHENTICATION_REQUIRED,
            Box::new(source),
        )
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl<E> From<E> for Error
where
    E: StdError + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, Box::new(error))
    }
}

impl From<Error> for Response {
    fn from(error: Error) -> Self {
        let mut respond_with_json = error.as_json;

        loop {
            if !respond_with_json {
                let mut response = Self::new(error.to_string().into());
                *response.status_mut() = error.status;
                break response;
            }

            match Self::build().status(error.status).json(&error) {
                Ok(response) => return response,
                Err(error) => {
                    respond_with_json = false;
                    // Placeholder for tracing...
                    if cfg!(debug_assertions) {
                        eprintln!("Error: {}", error);
                    }
                }
            }
        }
    }
}

impl From<Error> for http::Response<ResponseBody> {
    fn from(error: Error) -> Self {
        Response::from(error).into()
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Error", 1)?;

        // Serialize the error as a single element array containing an object with
        // a message field. We do this to provide compatibility with popular API
        // specification formats like GraphQL and JSON:API.
        if let Some(message) = &self.fmt {
            state.serialize_field(
                "errors",
                &[Message {
                    message: Cow::Borrowed(message),
                }],
            )?;
        } else {
            state.serialize_field(
                "errors",
                &[Message {
                    message: Cow::Owned(self.error.to_string()),
                }],
            )?;
        }

        state.end()
    }
}

impl Message<'static> {
    pub fn new(message: String) -> Self {
        Self {
            message: Cow::Owned(message),
        }
    }
}

impl StdError for Message<'_> {}

impl Display for Message<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
}

impl StdError for ServerError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(error) => error.source(),
            Self::Join(error) => error.source(),
            Self::Hyper(error) => error.source(),
        }
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, f),
            Self::Join(error) => Display::fmt(error, f),
            Self::Hyper(error) => Display::fmt(error, f),
        }
    }
}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<hyper::Error> for ServerError {
    fn from(error: hyper::Error) -> Self {
        Self::Hyper(error)
    }
}
