use crate::{error::Error, respond, Result, State, Value};
use bytes::buf::ext::{BufExt, Reader};
use http::header::{AsHeaderName, HeaderName, HeaderValue};
use http::{Method, Uri, Version};
use hyper::body::{Body as HyperBody, Buf};
use serde::de::DeserializeOwned;
use std::{io::Read, str::FromStr};

type Parameters = indexmap::IndexMap<&'static str, String>;
pub(crate) type Request = http::Request<Body>;

pub struct Body(pub(crate) HyperBody);

pub struct Context {
    pub(crate) parameters: Parameters,
    pub(crate) request: Request,
    pub(crate) state: State,
}

impl Body {
    // pub async fn bytes(&mut self) -> Result<Bytes, Error> {
    //     Ok(hyper::body::to_bytes(self.take()).await?)
    // }

    pub async fn json<'a, T: DeserializeOwned>(&'a mut self) -> Result<T> {
        let reader = self.reader().await?;
        serde_json::from_reader(reader).map_err(Error::from)
    }

    pub async fn text(&mut self) -> Result<String> {
        let mut reader = self.reader().await?;
        let mut value = String::new();

        reader.read_to_string(&mut value)?;
        Ok(value)
    }

    #[inline]
    async fn reader(&mut self) -> Result<Reader<impl Buf>> {
        let value = std::mem::replace(&mut self.0, Default::default());
        Ok(hyper::body::aggregate(value).await?.reader())
    }
}

impl Context {
    #[inline]
    pub fn body(&mut self) -> &mut Body {
        self.request.body_mut()
    }

    #[inline]
    pub fn accepts(&self, mime: &str) -> bool {
        self.header("accept").map_or(true, |accept| accept == mime)
    }

    #[inline]
    pub fn header(&self, name: impl AsHeaderName) -> Option<&HeaderValue> {
        self.request.headers().get(name)
    }

    #[inline]
    pub fn headers(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.request.headers().iter()
    }

    #[inline]
    pub fn global<T: Value>(&self) -> Result<&T, Error> {
        self.state.get()
    }

    #[inline]
    pub fn local<T: Value>(&self) -> Option<&T> {
        self.request.extensions().get()
    }

    #[inline]
    pub fn method(&self) -> &Method {
        self.request.method()
    }

    #[inline]
    pub fn param<T>(&self, name: &str) -> Result<T, Error>
    where
        Error: From<T::Err>,
        T: FromStr,
    {
        let value = match self.parameters.get(name) {
            Some(value) => value,
            None => todo!(),
        };

        match value.parse::<T>() {
            Ok(response) => Ok(response),
            Err(error) => Err(Error::from(error)),
        }
    }

    #[inline]
    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }

    #[inline]
    pub fn version(&self) -> Version {
        self.request.version()
    }
}

impl Context {
    #[inline]
    pub(crate) fn locate(&mut self) -> (&mut Parameters, &Method, &str) {
        (
            &mut self.parameters,
            self.request.method(),
            self.request.uri().path(),
        )
    }
}

impl From<(State, Request)> for Context {
    #[inline]
    fn from((state, request): (State, Request)) -> Context {
        Context {
            parameters: Parameters::new(),
            request,
            state,
        }
    }
}
