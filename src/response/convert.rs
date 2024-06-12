use futures::stream::StreamExt;
use http::{
    header::{HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    status::{InvalidStatusCode, StatusCode},
};
use http_body_util::StreamBody;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    response::{Body, Frame, Response},
    Error, Result,
};

pub trait IntoResponse: Sized {
    fn into_response(self) -> Result<Response>;

    fn with_header<K, V>(self, name: K, value: V) -> Result<Response>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        WithHeader::new(self, (name, value)).into_response()
    }

    fn with_status<T>(self, status: T) -> Result<Response>
    where
        StatusCode: TryFrom<T, Error = InvalidStatusCode>,
    {
        WithStatusCode::new(self, status).into_response()
    }
}

pub struct WithHeader<T: IntoResponse> {
    header: Result<(HeaderName, HeaderValue)>,
    value: T,
}

pub struct WithStatusCode<T: IntoResponse> {
    status: Result<StatusCode>,
    value: T,
}

impl<T: IntoResponse> WithHeader<T> {
    fn convert<K, V>(header: (K, V)) -> Result<(HeaderName, HeaderValue)>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        Ok((
            HeaderName::try_from(header.0)?,
            HeaderValue::try_from(header.1)?,
        ))
    }

    fn new<K, V>(value: T, header: (K, V)) -> WithHeader<T>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        WithHeader {
            header: Self::convert(header),
            value,
        }
    }
}

impl<T: IntoResponse> IntoResponse for WithHeader<T> {
    fn into_response(self) -> Result<Response> {
        let mut response = self.value.into_response()?;
        let (name, value) = self.header?;

        response.headers_mut().append(name, value);
        Ok(response)
    }
}

impl<T: IntoResponse> WithStatusCode<T> {
    fn convert<S>(status: S) -> Result<StatusCode>
    where
        StatusCode: TryFrom<S, Error = InvalidStatusCode>,
    {
        Ok(StatusCode::try_from(status)?)
    }

    fn new<S>(value: T, status: S) -> Self
    where
        StatusCode: TryFrom<S, Error = InvalidStatusCode>,
    {
        WithStatusCode {
            status: Self::convert(status),
            value,
        }
    }
}

impl<T: IntoResponse> IntoResponse for WithStatusCode<T> {
    fn into_response(self) -> Result<Response> {
        let mut response = self.value.into_response()?;

        *response.status_mut() = self.status?;
        Ok(response)
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response> {
        let mut response = Response::new(self);

        response.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );

        Ok(response)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response> {
        let mut response = Response::new(self);

        response.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );

        Ok(response)
    }
}

impl IntoResponse for File {
    fn into_response(self) -> Result<Response> {
        let stream = StreamBody::new(
            ReaderStream::new(self)
                .map(|result| result.map(Frame::data).map_err(Error::from))
                .boxed(),
        );

        Ok(Response::new(Body::Stream(stream)))
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response> {
        let mut response = Response::default();

        *response.status_mut() = StatusCode::NO_CONTENT;
        Ok(response)
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    Error: From<E>,
    T: IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        self?.into_response()
    }
}
