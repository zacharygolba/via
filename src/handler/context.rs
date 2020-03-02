use crate::{Error, Respond};
use bytes::buf::ext::{BufExt, Reader};
use http::header::{AsHeaderName, HeaderName, HeaderValue};
use http::{Extensions, Method, Request, Uri, Version};
use hyper::body::{Body as HyperBody, Buf};
use mime::Mime;
use serde::de::DeserializeOwned;
use std::{io::Read, str::FromStr, sync::Arc};

type Parameters = indexmap::IndexMap<&'static str, String>;

pub struct Body(HyperBody);

pub struct Context {
    pub(crate) parameters: Parameters,
    pub(crate) request: Request<Body>,
    pub state: Arc<Extensions>,
}

impl Body {
    // pub async fn bytes(&mut self) -> Result<Bytes, Error> {
    //     Ok(hyper::body::to_bytes(self.take()).await?)
    // }

    pub async fn json<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let reader = self.reader().await?;

        serde_json::from_reader(reader).map_err(|error| {
            let body = crate::json! {
                "error": {
                    "message": format!("{}", error),
                },
            };

            Error::from(error).catch(body.status(400))
        })
    }

    pub async fn text(&mut self) -> Result<String, Error> {
        let mut reader = self.reader().await?;
        let mut value = String::new();

        reader.read_to_string(&mut value)?;
        Ok(value)
    }

    #[inline]
    async fn reader(&mut self) -> Result<Reader<impl Buf>, Error> {
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
    pub fn local<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
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
    pub fn state<T>(&self) -> Result<&T, Error>
    where
        T: Send + Sync + 'static,
    {
        Ok(self.state.get::<T>().unwrap())
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
    pub(crate) fn new(state: Arc<Extensions>, request: Request<HyperBody>) -> Context {
        Context {
            parameters: Parameters::new(),
            request: request.map(Body),
            state,
        }
    }

    #[inline]
    pub(crate) fn locate(&mut self) -> (&mut Parameters, &Method, &str) {
        (
            &mut self.parameters,
            self.request.method(),
            self.request.uri().path(),
        )
    }
}
