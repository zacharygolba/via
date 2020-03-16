use crate::{error::Error, Result, State, Value};
use bytes::buf::ext::BufExt;
use http::header::{AsHeaderName, HeaderName, HeaderValue};
use http::{Method, Uri, Version};
use hyper::body::Body as HyperBody;
use serde::de::DeserializeOwned;
use std::{io::Read, mem::replace, str::FromStr};

type Parameters = indexmap::IndexMap<&'static str, String>;
pub(crate) type Request = http::Request<Body>;

pub struct Body(pub(crate) HyperBody);

pub struct Context {
    pub(crate) parameters: Parameters,
    pub(crate) request: Request,
    pub(crate) state: State,
}

impl Body {
    #[inline]
    pub async fn json<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let reader = self.read().await?;

        match serde_json::from_reader(reader) {
            Ok(value) => Ok(value),
            Err(e) => Err(e.into()),
        }
    }

    #[inline]
    pub async fn read(&mut self) -> Result<impl Read> {
        Ok(hyper::body::aggregate(self.take()).await?.reader())
    }

    #[inline]
    pub async fn text(&mut self) -> Result<String> {
        let mut value = String::new();

        self.read().await?.read_to_string(&mut value)?;
        Ok(value)
    }

    #[inline]
    fn take(&mut self) -> HyperBody {
        replace(&mut self.0, Default::default())
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
