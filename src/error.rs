//! Conviently work with errors that may occur in an application.
//!

use http::StatusCode;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};

use crate::response::Response;

/// A type alias for a boxed
/// [`Error`](std::error::Error)
/// that is `Send + Sync`.
///
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// An error type that can act as a specialized version of a
/// [`ResponseBuilder`](crate::response::ResponseBuilder).
///
#[derive(Debug)]
pub struct Error {
    as_json: bool,
    status: StatusCode,
    message: Option<String>,
    error: BoxError,
}

/// A serialized representation of an individual error.
///
struct SerializeError<'a> {
    message: &'a str,
}

impl Error {
    /// Returns a new [`Error`] with the provided message.
    ///
    #[inline]
    pub fn new(source: BoxError) -> Self {
        Self::internal_server_error(source)
    }

    /// Returns a new [`Error`] that will be serialized to JSON when converted to
    /// a [`Response`].
    ///
    #[inline]
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
    /// use via::{BoxError, ErrorBoundary, Next, Request};
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<(), BoxError> {
    ///     let mut app = via::new(());
    ///
    ///     // Add an `ErrorBoundary` middleware to the route tree that maps
    ///     // errors that occur in subsequent middleware by calling the `redact`
    ///     // function.
    ///     app.include(ErrorBoundary::map(|error, _| {
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
        match &self.message {
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
    #[inline]
    pub fn with_message(self, message: String) -> Self {
        Self {
            message: Some(message),
            ..self
        }
    }

    /// Sets the status code of that will be used when converted to a
    /// [`Response`].
    ///
    #[inline]
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
    /// use via::{BoxError, ErrorBoundary, Next, Request};
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<(), BoxError> {
    ///     let mut app = via::new(());
    ///
    ///     // Add an `ErrorBoundary` middleware to the route tree that maps
    ///     // errors that occur in subsequent middleware by calling the
    ///     // `use_canonical_reason` function.
    ///     app.include(ErrorBoundary::map(|error, _| {
    ///         // Prevent error messages that occur in downstream middleware from
    ///         // leaking into the response body by using the reason phrase of
    ///         // the status code associated with the error.
    ///         error.use_canonical_reason()
    ///     }));
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    #[inline]
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
    #[inline]
    pub fn bad_request(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::BAD_REQUEST, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `401 Unauthorized` status.
    ///
    #[inline]
    pub fn unauthorized(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::UNAUTHORIZED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `402 Payment Required` status.
    ///
    #[inline]
    pub fn payment_required(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::PAYMENT_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `403 Forbidden` status.
    ///
    #[inline]
    pub fn forbidden(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::FORBIDDEN, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `404 Not Found` status.
    ///
    #[inline]
    pub fn not_found(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::NOT_FOUND, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `405 Method Not Allowed` status.
    ///
    #[inline]
    pub fn method_not_allowed(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::METHOD_NOT_ALLOWED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `406 Not Acceptable` status.
    ///
    #[inline]
    pub fn not_acceptable(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::NOT_ACCEPTABLE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `407 Proxy Authentication Required` status.
    ///
    #[inline]
    pub fn proxy_authentication_required(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::PROXY_AUTHENTICATION_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `408 Request Timeout` status.
    ///
    #[inline]
    pub fn request_timeout(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::REQUEST_TIMEOUT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `409 Conflict` status.
    ///
    #[inline]
    pub fn conflict(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::CONFLICT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `410 Gone` status.
    ///
    #[inline]
    pub fn gone(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::GONE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `411 Length Required` status.
    ///
    #[inline]
    pub fn length_required(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::LENGTH_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `412 Precondition Failed` status.
    ///
    #[inline]
    pub fn precondition_failed(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::PRECONDITION_FAILED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `413 Payload Too Large` status.
    ///
    #[inline]
    pub fn payload_too_large(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::PAYLOAD_TOO_LARGE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `414 URI Too Long` status.
    ///
    #[inline]
    pub fn uri_too_long(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::URI_TOO_LONG, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `415 Unsupported Media Type` status.
    ///
    #[inline]
    pub fn unsupported_media_type(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::UNSUPPORTED_MEDIA_TYPE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `416 Range Not Satisfiable` status.
    ///
    #[inline]
    pub fn range_not_satisfiable(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::RANGE_NOT_SATISFIABLE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `417 Expectation Failed` status.
    ///
    #[inline]
    pub fn expectation_failed(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::EXPECTATION_FAILED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `418 I'm a teapot` status.
    ///
    #[inline]
    pub fn im_a_teapot(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::IM_A_TEAPOT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `421 Misdirected Request` status.
    ///
    #[inline]
    pub fn misdirected_request(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::MISDIRECTED_REQUEST, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `422 Unprocessable Entity` status.
    ///
    #[inline]
    pub fn unprocessable_entity(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::UNPROCESSABLE_ENTITY, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `423 Locked` status.
    ///
    #[inline]
    pub fn locked(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::LOCKED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `424 Failed Dependency` status.
    ///
    #[inline]
    pub fn failed_dependency(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::FAILED_DEPENDENCY, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `426 Upgrade Required` status.
    ///
    #[inline]
    pub fn upgrade_required(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::UPGRADE_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `428 Precondition Required` status.
    ///
    #[inline]
    pub fn precondition_required(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::PRECONDITION_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `429 Too Many Requests` status.
    ///
    #[inline]
    pub fn too_many_requests(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::TOO_MANY_REQUESTS, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `431 Request Header Fields Too Large` status.
    ///
    #[inline]
    pub fn request_header_fields_too_large(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `451 Unavailable For Legal Reasons` status.
    ///
    #[inline]
    pub fn unavailable_for_legal_reasons(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `500 Internal Server Error` status.
    ///
    #[inline]
    pub fn internal_server_error(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::INTERNAL_SERVER_ERROR, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `501 Not Implemented` status.
    ///
    #[inline]
    pub fn not_implemented(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::NOT_IMPLEMENTED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `502 Bad Gateway` status.
    ///
    #[inline]
    pub fn bad_gateway(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::BAD_GATEWAY, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `503 Service Unavailable` status.
    ///
    #[inline]
    pub fn service_unavailable(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::SERVICE_UNAVAILABLE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `504 Gateway Timeout` status.
    ///
    #[inline]
    pub fn gateway_timeout(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::GATEWAY_TIMEOUT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `505 HTTP Version Not Supported` status.
    ///
    #[inline]
    pub fn http_version_not_supported(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::HTTP_VERSION_NOT_SUPPORTED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `506 Variant Also Negotiates` status.
    ///
    #[inline]
    pub fn variant_also_negotiates(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::VARIANT_ALSO_NEGOTIATES, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `507 Insufficient Storage` status.
    ///
    #[inline]
    pub fn insufficient_storage(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::INSUFFICIENT_STORAGE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `508 Loop Detected` status.
    ///
    #[inline]
    pub fn loop_detected(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::LOOP_DETECTED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `510 Not Extended` status.
    ///
    #[inline]
    pub fn not_extended(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::NOT_EXTENDED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `511 Network Authentication Required` status.
    ///
    #[inline]
    pub fn network_authentication_required(source: BoxError) -> Self {
        Self::new_with_status(StatusCode::NETWORK_AUTHENTICATION_REQUIRED, source)
    }
}

impl Error {
    #[inline]
    fn new_with_status(status: StatusCode, source: BoxError) -> Self {
        Self {
            as_json: false,
            message: None,
            error: source,
            status,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl<T> From<T> for Error
where
    T: StdError + Send + Sync + 'static,
{
    #[inline]
    fn from(source: T) -> Self {
        Self {
            as_json: false,
            message: None,
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: Box::new(source),
        }
    }
}

impl From<Error> for Response {
    fn from(error: Error) -> Response {
        let mut response = if error.as_json {
            Response::json(&error).unwrap_or_else(|residual| {
                // Placeholder for tracing...
                if cfg!(debug_assertions) {
                    eprintln!("Error: {}", residual);
                }

                Response::text(error.to_string())
            })
        } else {
            Response::text(error.to_string())
        };

        response.set_status(error.status);
        response
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 1)?;

        // Serialize the error as a single element array containing an object with
        // a message field. We do this to provide compatibility with popular API
        // specification formats like GraphQL and JSON:API.
        if let Some(message) = &self.message {
            let errors = [SerializeError { message }];
            state.serialize_field("errors", &errors)?;
        } else {
            let message = self.error.to_string();
            let errors = [SerializeError { message: &message }];

            state.serialize_field("errors", &errors)?;
        }

        state.end()
    }
}

impl Serialize for SerializeError<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ErrorMessage", 1)?;

        state.serialize_field("message", &self.message)?;
        state.end()
    }
}
