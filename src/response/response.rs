use bytes::Bytes;
use cookie::CookieJar;
use http::header::SET_COOKIE;
use http::response::Parts;
use http::{HeaderMap, StatusCode, Version};
use http_body_util::combinators::BoxBody;
use std::fmt::{self, Debug, Formatter};

use super::ResponseBuilder;
use crate::body::{BufferBody, HttpBody};
use crate::error::{BoxError, Error};

pub struct Response {
    did_map: bool,
    cookies: CookieJar,
    response: http::Response<HttpBody<BufferBody>>,
}

impl Response {
    /// Consumes the response and returns a tuple containing the component
    /// parts of the response and the response body.
    ///
    #[inline]
    pub fn into_parts(self) -> (Parts, HttpBody<BufferBody>) {
        self.response.into_parts()
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the provided closure `map`.
    pub fn map<F>(self, map: F) -> Self
    where
        F: FnOnce(HttpBody<BufferBody>) -> BoxBody<Bytes, BoxError>,
    {
        if cfg!(debug_assertions) && self.did_map {
            // TODO: Replace this with tracing and a proper logger.
            eprintln!("calling response.map() more than once can create a reference cycle.");
        }

        Self {
            response: self.response.map(|body| HttpBody::Box(map(body))),
            did_map: true,
            ..self
        }
    }

    pub fn body(&self) -> &HttpBody<BufferBody> {
        self.response.body()
    }

    pub fn body_mut(&mut self) -> &mut HttpBody<BufferBody> {
        self.response.body_mut()
    }

    /// Returns a reference to the response cookies.
    ///
    pub fn cookies(&self) -> &CookieJar {
        &self.cookies
    }

    /// Returns a mutable reference to the response cookies.
    ///
    pub fn cookies_mut(&mut self) -> &mut CookieJar {
        &mut self.cookies
    }

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.response.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.response.headers_mut()
    }

    #[inline]
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn status_mut(&mut self) -> &mut StatusCode {
        self.response.status_mut()
    }

    #[inline]
    pub fn version(&self) -> Version {
        self.response.version()
    }
}

impl Response {
    #[inline]
    pub fn new(body: HttpBody<BufferBody>) -> Self {
        Self {
            did_map: false,
            cookies: CookieJar::new(),
            response: http::Response::new(body),
        }
    }

    #[inline]
    pub fn build() -> ResponseBuilder {
        Default::default()
    }

    #[inline]
    pub fn from_parts(parts: Parts, body: HttpBody<BufferBody>) -> Self {
        Self {
            did_map: false,
            cookies: CookieJar::new(),
            response: http::Response::from_parts(parts, body),
        }
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `400 Bad Request` status.
    ///
    pub fn bad_request() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::BAD_REQUEST)
            .text("Bad Request".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `401 Unauthorized` status.
    ///
    pub fn unauthorized() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::UNAUTHORIZED)
            .text("Unauthorized".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `402 Payment Required` status.
    ///
    pub fn payment_required() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::PAYMENT_REQUIRED)
            .text("Payment Required".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `403 Forbidden` status.
    ///
    pub fn forbidden() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::FORBIDDEN)
            .text("Forbidden".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `404 Not Found` status.
    ///
    pub fn not_found() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::NOT_FOUND)
            .text("Not Found".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `405 Method Not Allowed` status.
    ///
    pub fn method_not_allowed() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .text("Method Not Allowed".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `406 Not Acceptable` status.
    ///
    pub fn not_acceptable() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::NOT_ACCEPTABLE)
            .text("Not Acceptable".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `407 Proxy Authentication Required` status.
    ///
    pub fn proxy_authentication_required() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
            .text("Proxy Authentication Required".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `408 Request Timeout` status.
    ///
    pub fn request_timeout() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::REQUEST_TIMEOUT)
            .text("Request Timeout".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `409 Conflict` status.
    ///
    pub fn conflict() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::CONFLICT)
            .text("Conflict".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `410 Gone` status.
    ///
    pub fn gone() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::GONE)
            .text("Gone".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `411 Length Required` status.
    ///
    pub fn length_required() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::LENGTH_REQUIRED)
            .text("Length Required".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `412 Precondition Failed` status.
    ///
    pub fn precondition_failed() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::PRECONDITION_FAILED)
            .text("Precondition Failed".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `413 Payload Too Large` status.
    ///
    pub fn payload_too_large() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .text("Payload Too Large".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `414 URI Too Long` status.
    ///
    pub fn uri_too_long() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::URI_TOO_LONG)
            .text("URI Too Long".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `415 Unsupported Media Type` status.
    ///
    pub fn unsupported_media_type() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
            .text("Unsupported Media Type".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `416 Range Not Satisfiable` status.
    ///
    pub fn range_not_satisfiable() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .text("Range Not Satisfiable".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `417 Expectation Failed` status.
    ///
    pub fn expectation_failed() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::EXPECTATION_FAILED)
            .text("Expectation Failed".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `418 I'm a teapot` status.
    ///
    pub fn im_a_teapot() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::IM_A_TEAPOT)
            .text("Im A Teapot".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `421 Misdirected Request` status.
    ///
    pub fn misdirected_request() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::MISDIRECTED_REQUEST)
            .text("Misdirected Request".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `422 Unprocessable Entity` status.
    ///
    pub fn unprocessable_entity() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .text("Unprocessable Entity".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `423 Locked` status.
    ///
    pub fn locked() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::LOCKED)
            .text("Locked".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `424 Failed Dependency` status.
    ///
    pub fn failed_dependency() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::FAILED_DEPENDENCY)
            .text("Failed Dependency".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `426 Upgrade Required` status.
    ///
    pub fn upgrade_required() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::UPGRADE_REQUIRED)
            .text("Upgrade Required".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `428 Precondition Required` status.
    ///
    pub fn precondition_required() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::PRECONDITION_REQUIRED)
            .text("Precondition Required".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `429 Too Many Requests` status.
    ///
    pub fn too_many_requests() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .text("Too Many Requests".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `431 Request Header Fields Too Large` status.
    ///
    pub fn request_header_fields_too_large() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE)
            .text("Request Header Fields Too Large".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `451 Unavailable For Legal Reasons` status.
    ///
    pub fn unavailable_for_legal_reasons() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS)
            .text("Unavailable For Legal Reasons".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `500 Internal Server Error` status.
    ///
    pub fn internal_server_error() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .text("Internal Server Error".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `501 Not Implemented` status.
    ///
    pub fn not_implemented() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::NOT_IMPLEMENTED)
            .text("Not Implemented".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `502 Bad Gateway` status.
    ///
    pub fn bad_gateway() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::BAD_GATEWAY)
            .text("Bad Gateway".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `503 Service Unavailable` status.
    ///
    pub fn service_unavailable() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .text("Service Unavailable".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `504 Gateway Timeout` status.
    ///
    pub fn gateway_timeout() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::GATEWAY_TIMEOUT)
            .text("Gateway Timeout".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `505 HTTP Version Not Supported` status.
    ///
    pub fn http_version_not_supported() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::HTTP_VERSION_NOT_SUPPORTED)
            .text("HTTP Version Not Supported".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `506 Variant Also Negotiates` status.
    ///
    pub fn variant_also_negotiates() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::VARIANT_ALSO_NEGOTIATES)
            .text("Variant Also Negotiates".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `507 Insufficient Storage` status.
    ///
    pub fn insufficient_storage() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::INSUFFICIENT_STORAGE)
            .text("Insufficient Storage".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `508 Loop Detected` status.
    ///
    pub fn loop_detected() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::LOOP_DETECTED)
            .text("Loop Detected".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `510 Not Extended` status.
    ///
    pub fn not_extended() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::NOT_EXTENDED)
            .text("Not Extended".to_owned())
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `511 Network Authentication Required` status.
    ///
    pub fn network_authentication_required() -> Result<Self, Error> {
        Self::build()
            .status(StatusCode::NETWORK_AUTHENTICATION_REQUIRED)
            .text("Network Authentication Required".to_owned())
    }
}

impl Response {
    #[inline]
    pub(crate) fn from_inner(response: http::Response<HttpBody<BufferBody>>) -> Self {
        Self {
            response,
            did_map: false,
            cookies: CookieJar::new(),
        }
    }

    /// Consumes the response and returns the inner value after performing any
    /// final processing that may be required before the response is sent to the
    /// client.
    ///
    pub(crate) fn into_inner(self) -> http::Response<HttpBody<BufferBody>> {
        // Extract the component parts of the inner response.
        let (mut parts, body) = self.response.into_parts();

        // Map any cookies that have changed to an iter of "Set-Cookie" headers.
        let set_cookie_headers = self.cookies.delta().filter_map(|cookie| {
            match cookie.encoded().to_string().parse() {
                Ok(header) => Some((SET_COOKIE, header)),
                Err(error) => {
                    let _ = error;
                    // Placeholder for tracing...
                    None
                }
            }
        });

        // Extend the response headers with the "Set-Cookie" headers.
        parts.headers.extend(set_cookie_headers);

        // Reconstruct a http::Response from the component parts and response
        // body.
        //
        http::Response::from_parts(parts, body)
    }
}

impl Default for Response {
    fn default() -> Self {
        let empty = BufferBody::new(&[]);
        Self::new(HttpBody::Inline(empty))
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.response, f)
    }
}
