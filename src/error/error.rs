//! Conviently work with errors that may occur in an application.
//!

use http::StatusCode;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};

use super::{AnyError, Iter};
use crate::response::Response;

/// An error type that can be easily converted to a [`Response`].
///
#[derive(Debug)]
pub struct Error {
    as_json: bool,
    status: StatusCode,
    message: Option<String>,
    error: AnyError,
}

/// The serialized representation of an individual error.
///
#[derive(Debug, Serialize)]
struct ErrorMessage<'a> {
    message: &'a str,
}

impl Error {
    /// Returns a new [`Error`] with the provided message.
    ///
    #[inline]
    pub fn new(source: AnyError) -> Self {
        Self::internal_server_error(source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `400 Bad Request` status.
    ///
    #[inline]
    pub fn bad_request(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::BAD_REQUEST, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `401 Unauthorized` status.
    ///
    #[inline]
    pub fn unauthorized(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::UNAUTHORIZED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `402 Payment Required` status.
    ///
    #[inline]
    pub fn payment_required(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::PAYMENT_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `403 Forbidden` status.
    ///
    #[inline]
    pub fn forbidden(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::FORBIDDEN, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `404 Not Found` status.
    ///
    #[inline]
    pub fn not_found(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::NOT_FOUND, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `405 Method Not Allowed` status.
    ///
    #[inline]
    pub fn method_not_allowed(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::METHOD_NOT_ALLOWED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `406 Not Acceptable` status.
    ///
    #[inline]
    pub fn not_acceptable(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::NOT_ACCEPTABLE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `407 Proxy Authentication Required` status.
    ///
    #[inline]
    pub fn proxy_authentication_required(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::PROXY_AUTHENTICATION_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `408 Request Timeout` status.
    ///
    #[inline]
    pub fn request_timeout(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::REQUEST_TIMEOUT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `409 Conflict` status.
    ///
    #[inline]
    pub fn conflict(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::CONFLICT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `410 Gone` status.
    ///
    #[inline]
    pub fn gone(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::GONE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `411 Length Required` status.
    ///
    #[inline]
    pub fn length_required(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::LENGTH_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `412 Precondition Failed` status.
    ///
    #[inline]
    pub fn precondition_failed(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::PRECONDITION_FAILED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `413 Payload Too Large` status.
    ///
    #[inline]
    pub fn payload_too_large(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::PAYLOAD_TOO_LARGE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `414 URI Too Long` status.
    ///
    #[inline]
    pub fn uri_too_long(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::URI_TOO_LONG, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `415 Unsupported Media Type` status.
    ///
    #[inline]
    pub fn unsupported_media_type(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::UNSUPPORTED_MEDIA_TYPE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `416 Range Not Satisfiable` status.
    ///
    #[inline]
    pub fn range_not_satisfiable(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::RANGE_NOT_SATISFIABLE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `417 Expectation Failed` status.
    ///
    #[inline]
    pub fn expectation_failed(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::EXPECTATION_FAILED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `418 I'm a teapot` status.
    ///
    #[inline]
    pub fn im_a_teapot(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::IM_A_TEAPOT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `421 Misdirected Request` status.
    ///
    #[inline]
    pub fn misdirected_request(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::MISDIRECTED_REQUEST, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `422 Unprocessable Entity` status.
    ///
    #[inline]
    pub fn unprocessable_entity(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::UNPROCESSABLE_ENTITY, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `423 Locked` status.
    ///
    #[inline]
    pub fn locked(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::LOCKED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `424 Failed Dependency` status.
    ///
    #[inline]
    pub fn failed_dependency(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::FAILED_DEPENDENCY, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `426 Upgrade Required` status.
    ///
    #[inline]
    pub fn upgrade_required(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::UPGRADE_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `428 Precondition Required` status.
    ///
    #[inline]
    pub fn precondition_required(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::PRECONDITION_REQUIRED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `429 Too Many Requests` status.
    ///
    #[inline]
    pub fn too_many_requests(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::TOO_MANY_REQUESTS, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `431 Request Header Fields Too Large` status.
    ///
    #[inline]
    pub fn request_header_fields_too_large(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `451 Unavailable For Legal Reasons` status.
    ///
    #[inline]
    pub fn unavailable_for_legal_reasons(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `500 Internal Server Error` status.
    ///
    #[inline]
    pub fn internal_server_error(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::INTERNAL_SERVER_ERROR, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `501 Not Implemented` status.
    ///
    #[inline]
    pub fn not_implemented(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::NOT_IMPLEMENTED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `502 Bad Gateway` status.
    ///
    #[inline]
    pub fn bad_gateway(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::BAD_GATEWAY, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `503 Service Unavailable` status.
    ///
    #[inline]
    pub fn service_unavailable(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::SERVICE_UNAVAILABLE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `504 Gateway Timeout` status.
    ///
    #[inline]
    pub fn gateway_timeout(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::GATEWAY_TIMEOUT, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `505 HTTP Version Not Supported` status.
    ///
    #[inline]
    pub fn http_version_not_supported(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::HTTP_VERSION_NOT_SUPPORTED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `506 Variant Also Negotiates` status.
    ///
    #[inline]
    pub fn variant_also_negotiates(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::VARIANT_ALSO_NEGOTIATES, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `507 Insufficient Storage` status.
    ///
    #[inline]
    pub fn insufficient_storage(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::INSUFFICIENT_STORAGE, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `508 Loop Detected` status.
    ///
    #[inline]
    pub fn loop_detected(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::LOOP_DETECTED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `510 Not Extended` status.
    ///
    #[inline]
    pub fn not_extended(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::NOT_EXTENDED, source)
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `511 Network Authentication Required` status.
    ///
    #[inline]
    pub fn network_authentication_required(source: AnyError) -> Self {
        Self::new_with_status(StatusCode::NETWORK_AUTHENTICATION_REQUIRED, source)
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

    /// Returns a new [`Error`] that will use the provided message instead of
    /// calling the [`Display`] implementation of the error source when
    /// converted to a [`Response`].
    ///
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
        if cfg!(debug_assertions) {
            if status.is_informational() {
                eprintln!("Warning: Setting an information status code on an error");
            }

            if status.is_redirection() {
                eprintln!("Warning: Setting a redirect status code on an error");
            }

            if status.is_success() {
                eprintln!("Warning: Setting a success status code on an error");
            }
        }

        Self { status, ..self }
    }

    /// Returns a new [`Error`] that will use the canonical reason phrase of the
    /// status code as the message included in the [`Response`] body that is
    /// generated when converted to a [`Response`].
    ///
    #[inline]
    pub fn use_canonical_reason(self) -> Self {
        if let Some(reason) = self.status.canonical_reason() {
            let message = Some(reason.to_owned());
            return Self { message, ..self };
        }

        // Placeholder for tracing...

        self
    }
}

impl Error {
    /// Returns an iterator over the sources of this error.
    ///
    pub fn iter(&self) -> Iter {
        Iter::new(Some(self.source()))
    }

    /// Returns a reference to the error source.
    ///
    pub fn source(&self) -> &(dyn StdError + 'static) {
        &*self.error
    }
}

impl Error {
    #[inline]
    fn new_with_status(status: StatusCode, source: AnyError) -> Self {
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
        Self::new(Box::new(source))
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
            let errors = [ErrorMessage { message }];
            state.serialize_field("errors", &errors)?;
        } else {
            let message = self.error.to_string();
            let errors = [ErrorMessage { message: &message }];

            state.serialize_field("errors", &errors)?;
        }

        state.end()
    }
}
