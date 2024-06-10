use bytes::Buf;
use http::{uri::PathAndQuery, HeaderMap, Method, Uri, Version};
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, io::Read, str::FromStr};

use crate::{Error, HttpRequest, Result};

type Request = http::Request<Body>;

pub(crate) type PathParams = HashMap<&'static str, (usize, usize)>;

#[derive(Debug)]
pub struct Body {
    value: Option<Incoming>,
}

#[derive(Debug)]
pub struct Context {
    request: Request,
    params: PathParams,
}

#[derive(Clone, Copy, Debug)]
pub struct PathParam<'a> {
    name: &'a str,
    value: Option<&'a str>,
}

impl Body {
    pub async fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let buf = self.aggregate().await?;
        let mut bytes = Vec::with_capacity(buf.remaining());

        buf.reader().read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub async fn read_json<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let reader = self.aggregate().await?.reader();

        match serde_json::from_reader(reader) {
            Ok(json) => Ok(json),
            Err(e) => Err(Error::from(e).status(400).json()),
        }
    }

    pub async fn read_text(&mut self) -> Result<String> {
        let bytes = self.read_bytes().await?;
        Ok(String::from_utf8(bytes)?)
    }

    async fn aggregate(&mut self) -> Result<impl Buf> {
        if let Some(value) = self.value.take() {
            Ok(value.collect().await?.aggregate())
        } else {
            Ok(Empty::new().collect().await?.aggregate())
        }
    }
}

impl Context {
    pub fn body(&self) -> &Body {
        self.request.body()
    }

    pub fn body_mut(&mut self) -> &mut Body {
        self.request.body_mut()
    }

    pub fn get<T>(&self) -> Result<&T>
    where
        T: Send + Sync + 'static,
    {
        match self.request.extensions().get() {
            Some(value) => Ok(value),
            None => crate::bail!("unknown type"),
        }
    }

    pub fn headers(&self) -> &HeaderMap {
        self.request.headers()
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

    pub fn param<'a>(&'a self, name: &'a str) -> PathParam<'a> {
        PathParam {
            name,
            value: self
                .params
                .get(name)
                .copied()
                .map(|(start, end)| &self.path()[start..end]),
        }
    }

    pub fn path_and_query(&self) -> Option<&PathAndQuery> {
        self.request.uri().path_and_query()
    }

    pub fn path(&self) -> &str {
        self.request.uri().path()
    }

    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }

    pub fn version(&self) -> Version {
        self.request.version()
    }

    pub(crate) fn new(request: HttpRequest, params: PathParams) -> Self {
        Context {
            request: request.map(|value| Body { value: Some(value) }),
            params,
        }
    }
}

// TODO:
// Explore alternative ways to handle request parameters or take inspiration
// from the API of `std::option::Option` or `std::result::Result`.
impl<'a> PathParam<'a> {
    pub fn parse<T>(&self) -> Result<T>
    where
        Error: From<<T as FromStr>::Err>,
        T: FromStr,
    {
        Ok(self.required()?.parse()?)
    }

    pub fn expect(&self, message: &str) -> Result<&'a str> {
        self.value
            .ok_or_else(|| Error::new(message.to_owned(), 400))
    }

    pub fn required(&self) -> Result<&'a str> {
        self.value.ok_or_else(|| {
            Error::new(
                format!("missing required path parameter: \"{}\"", self.name),
                400,
            )
        })
    }
}
