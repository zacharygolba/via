use crate::{Error, Result};
use bytes::{buf::ext::BufExt, Buf, Bytes};
use futures::Stream;
use http::header::{self, AsHeaderName, HeaderMap, HeaderName, HeaderValue};
use http::{Method, Uri, Version};
use hyper::body::{aggregate, Body as HyperBody};
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

type Request = http::Request<HyperBody>;

pub struct Body(HyperBody);

pub struct Context {
    pub(super) request: Request,
    pub(super) state: State,
}

#[derive(Clone, Copy)]
pub struct Headers<'a> {
    entries: &'a HeaderMap,
}

#[derive(Default, Clone)]
pub struct Parameters {
    entries: IndexMap<&'static str, String>,
}

#[derive(Default)]
pub(super) struct State {
    pub(super) params: Parameters,
}

impl Body {
    pub async fn json<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let reader = aggregate(self.0).await?.reader();

        match serde_json::from_reader(reader) {
            Ok(value) => Ok(value),
            Err(e) => Err(Error::from(e).status(400).json()),
        }
    }

    pub async fn text(self) -> Result<String> {
        let src = aggregate(self.0).await?;
        let mut dest = String::with_capacity(src.remaining());

        src.reader().read_to_string(&mut dest)?;
        Ok(dest)
    }

    pub async fn vec(self) -> Result<Vec<u8>> {
        let src = aggregate(self.0).await?;
        let mut dest = Vec::with_capacity(src.remaining());

        src.reader().read_to_end(&mut dest)?;
        Ok(dest)
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Stream for Body {
    type Item = Result<Bytes>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut task::Context,
    ) -> Poll<Option<Self::Item>> {
        match Stream::poll_next(Pin::new(&mut self.0), context) {
            Poll::Ready(option) => Poll::Ready(option.map(|result| Ok(result?))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Context {
    pub fn get<T>(&self) -> Result<&T>
    where
        T: Send + Sync + 'static,
    {
        match self.request.extensions().get() {
            Some(value) => Ok(value),
            None => crate::bail!("unknown type"),
        }
    }

    pub fn headers(&self) -> Headers {
        Headers {
            entries: self.request.headers(),
        }
    }

    pub fn insert<T>(&mut self, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.request.extensions_mut().insert(value);
    }

    pub fn method(&self) -> &Method {
        self.request.method()
    }

    pub fn params(&self) -> &Parameters {
        &self.state.params
    }

    pub fn read(&mut self) -> Body {
        let body = self.request.body_mut();
        let empty = HyperBody::empty();

        Body(replace(body, empty))
    }

    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }

    pub fn version(&self) -> Version {
        self.request.version()
    }
}

#[doc(hidden)]
impl Context {
    pub fn locate(&mut self) -> (&mut Parameters, &Method, &str) {
        (
            &mut self.state.params,
            self.request.method(),
            self.request.uri().path(),
        )
    }
}

#[doc(hidden)]
impl From<Request> for Context {
    fn from(request: Request) -> Self {
        Context {
            request,
            state: Default::default(),
        }
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
            crate::bail!(r#"unknown parameter "{}""#, name)
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
