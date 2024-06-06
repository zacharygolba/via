// pub mod cookies;

use crate::{Error, Result};
use bytes::Buf;
use http::header::{self, AsHeaderName, HeaderMap, HeaderName, HeaderValue};
use http::{Method, Uri, Version};
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::{
    fmt::{self, Debug, Formatter},
    mem::replace,
    str::FromStr,
    // task::{self, Poll},
};

type Request = http::Request<Body>;

pub struct Body(BodyState);

#[derive(Debug)]
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

#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) params: Parameters,
}

#[derive(Debug)]
enum BodyState {
    Empty(Empty<Bytes>),
    Incoming(Incoming),
}

impl Body {
    pub async fn json<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let reader = self.aggregate().await?.reader();
        serde_json::from_reader(reader).map_err(|e| Error::from(e).status(400).json())
    }

    pub async fn text(self) -> Result<String> {
        let bytes = self.vec().await?;
        Ok(String::from_utf8(bytes)?)
    }

    pub async fn vec(self) -> Result<Vec<u8>> {
        let buf = self.aggregate().await?;
        let mut bytes = Vec::with_capacity(buf.remaining());

        buf.reader().read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

impl Body {
    fn incoming(incoming: Incoming) -> Self {
        Body(BodyState::Incoming(incoming))
    }

    fn empty() -> Self {
        Body(BodyState::Empty(Empty::new()))
    }

    async fn aggregate(self) -> Result<impl Buf> {
        Ok(match self.0 {
            BodyState::Empty(empty) => empty.collect().await?.aggregate(),
            BodyState::Incoming(incoming) => incoming.collect().await?.aggregate(),
        })
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

// impl Stream for Body {
//     type Item = Result<Bytes>;

//     fn poll_next(
//         mut self: Pin<&mut Self>,
//         context: &mut task::Context,
//     ) -> Poll<Option<Self::Item>> {
//         match Stream::poll_next(Pin::new(&mut self.0), context) {
//             Poll::Ready(option) => Poll::Ready(option.map(|result| Ok(result?))),
//             Poll::Pending => Poll::Pending,
//         }
//     }
// }

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
        T: Clone + Send + Sync + 'static,
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
        replace(self.request.body_mut(), Body::empty())
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

#[doc(hidden)]
impl From<crate::HttpRequest> for Context {
    fn from(request: crate::HttpRequest) -> Self {
        Context {
            request: request.map(Body::incoming),
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
