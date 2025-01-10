use bytes::{BufMut, Bytes, BytesMut};
use cookie::CookieJar;
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, SET_COOKIE, TRANSFER_ENCODING};
use http::response::Parts;
use http::{HeaderMap, StatusCode, Version};
use http_body::Frame;
use http_body_util::combinators::BoxBody;
use http_body_util::StreamBody;
use serde::Serialize;
use std::fmt::{self, Debug, Formatter};

use super::ResponseBuilder;
use super::{APPLICATION_JSON, CHUNKED_ENCODING, TEXT_HTML, TEXT_PLAIN};
use crate::body::{BufferBody, HttpBody};
use crate::error::{BoxError, Error};

pub struct Response {
    did_map: bool,
    cookies: Box<CookieJar>,
    response: http::Response<HttpBody<BufferBody>>,
}

impl Response {
    /// Consumes the response and returns a tuple containing the component
    /// parts of the response and the response body.
    ///
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

    pub fn headers(&self) -> &HeaderMap {
        self.response.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.response.headers_mut()
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn status_mut(&mut self) -> &mut StatusCode {
        self.response.status_mut()
    }

    /// A shorthand method for `*self.status_mut() = status`.
    ///
    pub fn set_status(&mut self, status: StatusCode) {
        *self.response.status_mut() = status;
    }

    pub fn version(&self) -> Version {
        self.response.version()
    }
}

impl Response {
    #[inline]
    pub fn new(body: HttpBody<BufferBody>) -> Self {
        Self {
            did_map: false,
            cookies: Box::new(CookieJar::new()),
            response: http::Response::new(body),
        }
    }

    #[inline]
    pub fn build() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    #[inline]
    pub fn html(body: String) -> Self {
        let body = BufferBody::new(body.as_bytes());
        let len = body.len();

        let mut response = Self::new(HttpBody::Inline(body));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, TEXT_HTML);
        headers.insert(CONTENT_LENGTH, len.into());

        response
    }

    #[inline]
    pub fn text(body: String) -> Self {
        let body = BufferBody::new(body.as_bytes());
        let len = body.len();

        let mut response = Self::new(HttpBody::Inline(body));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, TEXT_PLAIN);
        headers.insert(CONTENT_LENGTH, len.into());

        response
    }

    #[inline]
    pub fn json<T>(body: &T) -> Result<Response, Error>
    where
        T: Serialize,
    {
        let (len, json) = {
            let mut writer = BytesMut::new().writer();
            serde_json::to_writer(&mut writer, body)?;

            let buf = writer.into_inner().freeze();
            (buf.len(), BufferBody::from_raw(buf))
        };

        let mut response = Self::new(HttpBody::Inline(json));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, APPLICATION_JSON);
        headers.insert(CONTENT_LENGTH, len.into());

        Ok(response)
    }

    /// Create a response from a stream of `Result<Frame<Bytes>, BoxError>`.
    ///
    #[inline]
    pub fn stream<T, E>(stream: T) -> Self
    where
        T: Stream<Item = Result<Frame<Bytes>, BoxError>> + Send + Sync + 'static,
    {
        let body = BoxBody::new(StreamBody::new(stream));

        let mut response = Self::new(HttpBody::Box(body));
        let headers = response.headers_mut();

        headers.insert(TRANSFER_ENCODING, CHUNKED_ENCODING);

        response
    }

    #[inline]
    pub fn from_parts(parts: Parts, body: HttpBody<BufferBody>) -> Self {
        Self {
            did_map: false,
            cookies: Box::new(CookieJar::new()),
            response: http::Response::from_parts(parts, body),
        }
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `400 Bad Request` status.
    ///
    #[inline]
    pub fn bad_request() -> Self {
        let mut this = Self::text("Bad Request".to_owned());

        *this.response.status_mut() = StatusCode::BAD_REQUEST;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `401 Unauthorized` status.
    ///
    #[inline]
    pub fn unauthorized() -> Self {
        let mut this = Self::text("Unauthorized".to_owned());

        *this.response.status_mut() = StatusCode::UNAUTHORIZED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `402 Payment Required` status.
    ///
    #[inline]
    pub fn payment_required() -> Self {
        let mut this = Self::text("Payment Required".to_owned());

        *this.response.status_mut() = StatusCode::PAYMENT_REQUIRED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `403 Forbidden` status.
    ///
    #[inline]
    pub fn forbidden() -> Self {
        let mut this = Self::text("Forbidden".to_owned());

        *this.response.status_mut() = StatusCode::FORBIDDEN;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `404 Not Found` status.
    ///
    #[inline]
    pub fn not_found() -> Self {
        let mut this = Self::text("Not Found".to_owned());

        *this.response.status_mut() = StatusCode::NOT_FOUND;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `405 Method Not Allowed` status.
    ///
    #[inline]
    pub fn method_not_allowed() -> Self {
        let mut this = Self::text("Method Not Allowed".to_owned());

        *this.response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `406 Not Acceptable` status.
    ///
    #[inline]
    pub fn not_acceptable() -> Self {
        let mut this = Self::text("Not Acceptable".to_owned());

        *this.response.status_mut() = StatusCode::NOT_ACCEPTABLE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `407 Proxy Authentication Required` status.
    ///
    #[inline]
    pub fn proxy_authentication_required() -> Self {
        let mut this = Self::text("Proxy Authentication Required".to_owned());

        *this.response.status_mut() = StatusCode::PROXY_AUTHENTICATION_REQUIRED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `408 Request Timeout` status.
    ///
    #[inline]
    pub fn request_timeout() -> Self {
        let mut this = Self::text("Request Timeout".to_owned());

        *this.response.status_mut() = StatusCode::REQUEST_TIMEOUT;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `409 Conflict` status.
    ///
    #[inline]
    pub fn conflict() -> Self {
        let mut this = Self::text("Conflict".to_owned());

        *this.response.status_mut() = StatusCode::CONFLICT;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `410 Gone` status.
    ///
    #[inline]
    pub fn gone() -> Self {
        let mut this = Self::text("Gone".to_owned());

        *this.response.status_mut() = StatusCode::GONE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `411 Length Required` status.
    ///
    #[inline]
    pub fn length_required() -> Self {
        let mut this = Self::text("Length Required".to_owned());

        *this.response.status_mut() = StatusCode::LENGTH_REQUIRED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `412 Precondition Failed` status.
    ///
    #[inline]
    pub fn precondition_failed() -> Self {
        let mut this = Self::text("Precondition Failed".to_owned());

        *this.response.status_mut() = StatusCode::PRECONDITION_FAILED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `413 Payload Too Large` status.
    ///
    #[inline]
    pub fn payload_too_large() -> Self {
        let mut this = Self::text("Payload Too Large".to_owned());

        *this.response.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `414 URI Too Long` status.
    ///
    #[inline]
    pub fn uri_too_long() -> Self {
        let mut this = Self::text("URI Too Long".to_owned());

        *this.response.status_mut() = StatusCode::URI_TOO_LONG;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `415 Unsupported Media Type` status.
    ///
    #[inline]
    pub fn unsupported_media_type() -> Self {
        let mut this = Self::text("Unsupported Media Type".to_owned());

        *this.response.status_mut() = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `416 Range Not Satisfiable` status.
    ///
    #[inline]
    pub fn range_not_satisfiable() -> Self {
        let mut this = Self::text("Range Not Satisfiable".to_owned());

        *this.response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `417 Expectation Failed` status.
    ///
    #[inline]
    pub fn expectation_failed() -> Self {
        let mut this = Self::text("Expectation Failed".to_owned());

        *this.response.status_mut() = StatusCode::EXPECTATION_FAILED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `418 I'm a teapot` status.
    ///
    #[inline]
    pub fn im_a_teapot() -> Self {
        let mut this = Self::text("Im A Teapot".to_owned());

        *this.response.status_mut() = StatusCode::IM_A_TEAPOT;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `421 Misdirected Request` status.
    ///
    #[inline]
    pub fn misdirected_request() -> Self {
        let mut this = Self::text("Misdirected Request".to_owned());

        *this.response.status_mut() = StatusCode::MISDIRECTED_REQUEST;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `422 Unprocessable Entity` status.
    ///
    #[inline]
    pub fn unprocessable_entity() -> Self {
        let mut this = Self::text("Unprocessable Entity".to_owned());

        *this.response.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `423 Locked` status.
    ///
    #[inline]
    pub fn locked() -> Self {
        let mut this = Self::text("Locked".to_owned());

        *this.response.status_mut() = StatusCode::LOCKED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `424 Failed Dependency` status.
    ///
    #[inline]
    pub fn failed_dependency() -> Self {
        let mut this = Self::text("Failed Dependency".to_owned());

        *this.response.status_mut() = StatusCode::FAILED_DEPENDENCY;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `426 Upgrade Required` status.
    ///
    #[inline]
    pub fn upgrade_required() -> Self {
        let mut this = Self::text("Upgrade Required".to_owned());

        *this.response.status_mut() = StatusCode::UPGRADE_REQUIRED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `428 Precondition Required` status.
    ///
    #[inline]
    pub fn precondition_required() -> Self {
        let mut this = Self::text("Precondition Required".to_owned());

        *this.response.status_mut() = StatusCode::PRECONDITION_REQUIRED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `429 Too Many Requests` status.
    ///
    #[inline]
    pub fn too_many_requests() -> Self {
        let mut this = Self::text("Too Many Requests".to_owned());

        *this.response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `431 Request Header Fields Too Large` status.
    ///
    #[inline]
    pub fn request_header_fields_too_large() -> Self {
        let mut this = Self::text("Request Header Fields Too Large".to_owned());

        *this.response.status_mut() = StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `451 Unavailable For Legal Reasons` status.
    ///
    #[inline]
    pub fn unavailable_for_legal_reasons() -> Self {
        let mut this = Self::text("Unavailable For Legal Reasons".to_owned());

        *this.response.status_mut() = StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `500 Internal Server Error` status.
    ///
    #[inline]
    pub fn internal_server_error() -> Self {
        let mut this = Self::text("Internal Server Error".to_owned());

        *this.response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `501 Not Implemented` status.
    ///
    #[inline]
    pub fn not_implemented() -> Self {
        let mut this = Self::text("Not Implemented".to_owned());

        *this.response.status_mut() = StatusCode::NOT_IMPLEMENTED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `502 Bad Gateway` status.
    ///
    #[inline]
    pub fn bad_gateway() -> Self {
        let mut this = Self::text("Bad Gateway".to_owned());

        *this.response.status_mut() = StatusCode::BAD_GATEWAY;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `503 Service Unavailable` status.
    ///
    #[inline]
    pub fn service_unavailable() -> Self {
        let mut this = Self::text("Service Unavailable".to_owned());

        *this.response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `504 Gateway Timeout` status.
    ///
    #[inline]
    pub fn gateway_timeout() -> Self {
        let mut this = Self::text("Gateway Timeout".to_owned());

        *this.response.status_mut() = StatusCode::GATEWAY_TIMEOUT;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `505 HTTP Version Not Supported` status.
    ///
    #[inline]
    pub fn http_version_not_supported() -> Self {
        let mut this = Self::text("HTTP Version Not Supported".to_owned());

        *this.response.status_mut() = StatusCode::HTTP_VERSION_NOT_SUPPORTED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `506 Variant Also Negotiates` status.
    ///
    #[inline]
    pub fn variant_also_negotiates() -> Self {
        let mut this = Self::text("Variant Also Negotiates".to_owned());

        *this.response.status_mut() = StatusCode::VARIANT_ALSO_NEGOTIATES;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `507 Insufficient Storage` status.
    ///
    #[inline]
    pub fn insufficient_storage() -> Self {
        let mut this = Self::text("Insufficient Storage".to_owned());

        *this.response.status_mut() = StatusCode::INSUFFICIENT_STORAGE;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `508 Loop Detected` status.
    ///
    #[inline]
    pub fn loop_detected() -> Self {
        let mut this = Self::text("Loop Detected".to_owned());

        *this.response.status_mut() = StatusCode::LOOP_DETECTED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `510 Not Extended` status.
    ///
    #[inline]
    pub fn not_extended() -> Self {
        let mut this = Self::text("Not Extended".to_owned());

        *this.response.status_mut() = StatusCode::NOT_EXTENDED;
        this
    }

    /// Returns a new [`Error`] from the provided source that will generate a
    /// [`Response`] with a `511 Network Authentication Required` status.
    ///
    #[inline]
    pub fn network_authentication_required() -> Self {
        let mut this = Self::text("Network Authentication Required".to_owned());

        *this.response.status_mut() = StatusCode::NETWORK_AUTHENTICATION_REQUIRED;
        this
    }
}

impl Response {
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

impl From<http::Response<HttpBody<BufferBody>>> for Response {
    fn from(response: http::Response<HttpBody<BufferBody>>) -> Self {
        Self {
            response,
            did_map: false,
            cookies: Box::new(CookieJar::new()),
        }
    }
}
