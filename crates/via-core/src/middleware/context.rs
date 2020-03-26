use crate::{Error, Result};
use bytes::{buf::ext::BufExt, Buf, Bytes};
use futures::Stream;
use http::header::{self, AsHeaderName, HeaderMap, HeaderName, HeaderValue};
use http::{Method, Request, Uri, Version};
use hyper::{body::aggregate, Body as HyperBody};
use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use std::{
    fmt::{self, Debug, Formatter},
    io::Read,
    mem::replace,
    pin::Pin,
    str::FromStr,
    task::{self, Poll},
};

pub struct Body {
    value: HyperBody,
}

pub struct Context {
    params: Parameters,
    request: Request<HyperBody>,
}

#[derive(Clone, Copy)]
pub struct Headers<'a> {
    entries: &'a HeaderMap,
}

pub struct Parameters {
    entries: IndexMap<&'static str, String>,
}

impl Body {
    pub async fn json<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let reader = aggregate(self.value).await?.reader();

        match serde_json::from_reader(reader) {
            Ok(value) => Ok(value),
            Err(e) => Err(Error::from(e).status(400).json()),
        }
    }

    pub async fn text(self) -> Result<String> {
        let src = aggregate(self.value).await?;
        let mut dest = String::with_capacity(src.remaining());

        src.reader().read_to_string(&mut dest)?;
        Ok(dest)
    }

    pub async fn vec(self) -> Result<Vec<u8>> {
        let src = aggregate(self.value).await?;
        let mut dest = Vec::with_capacity(src.remaining());

        src.reader().read_to_end(&mut dest)?;
        Ok(dest)
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

impl Stream for Body {
    type Item = Result<Bytes>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut task::Context,
    ) -> Poll<Option<Self::Item>> {
        match Stream::poll_next(Pin::new(&mut self.value), context) {
            Poll::Ready(option) => Poll::Ready(option.map(|result| Ok(result?))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Context {
    pub(crate) fn locate(context: &mut Context) -> (&mut Parameters, &Method, &str) {
        let Context { params, request } = context;
        (params, request.method(), request.uri().path())
    }

    pub fn headers(&self) -> Headers {
        Headers {
            entries: self.request.headers(),
        }
    }

    pub fn method(&self) -> &Method {
        self.request.method()
    }

    pub fn params(&self) -> &Parameters {
        &self.params
    }

    pub fn read(&mut self) -> Body {
        let body = self.request.body_mut();
        let empty = HyperBody::empty();

        Body {
            value: replace(body, empty),
        }
    }

    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }

    pub fn version(&self) -> Version {
        self.request.version()
    }
}

impl<'a> Headers<'a> {
    pub fn get(&self, name: impl AsHeaderName) -> Option<&'a HeaderValue> {
        self.entries.get(name)
    }

    pub fn iter(&self) -> header::Iter<'a, HeaderValue> {
        self.into_iter()
    }
}

impl<'a> Debug for Headers<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a> IntoIterator for Headers<'a> {
    type IntoIter = header::Iter<'a, HeaderValue>;
    type Item = (&'a HeaderName, &'a HeaderValue);

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl Parameters {
    pub fn get<T>(&self, name: &str) -> Result<T>
    where
        Error: From<T::Err>,
        T: FromStr,
    {
        if let Some(value) = self.entries.get(name) {
            Ok(value.parse()?)
        } else {
            error::bail!(r#"unknown parameter "{}""#, name)
        }
    }

    pub(crate) fn insert(&mut self, name: &'static str, value: String) {
        self.entries.insert(name, value);
    }
}

impl Debug for Parameters {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.entries, f)
    }
}
