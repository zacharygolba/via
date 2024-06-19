use http::{HeaderMap, Method, Uri, Version};

use super::{
    query_parser::{ParsedQueryParams, QueryParams},
    Body, IncomingRequest, PathParam, PathParams,
};
use crate::{Error, Result};

#[derive(Debug)]
pub struct Context {
    request: http::Request<Body>,
    path_params: PathParams,
    query_params: ParsedQueryParams,
}

impl Context {
    pub(crate) fn new(request: IncomingRequest, path_params: PathParams) -> Self {
        let query_params = if let Some(query) = request.uri().query() {
            super::query_parser::parse(query)
        } else {
            ParsedQueryParams::default()
        };

        Context {
            request: request.map(Body::new),
            path_params,
            query_params,
        }
    }

    /// Returns a reference to the body associated with the request.
    pub fn body(&self) -> &Body {
        self.request.body()
    }

    /// Returns a mutable reference to the body associated with the request.
    pub fn body_mut(&mut self) -> &mut Body {
        self.request.body_mut()
    }

    pub fn get<T>(&self) -> Result<&T>
    where
        T: Send + Sync + 'static,
    {
        if let Some(value) = self.request.extensions().get() {
            Ok(value)
        } else {
            Err(Error::new("unknown type".to_owned()))
        }
    }

    /// Returns a reference to a map that contains the headers associated with
    /// the request.
    pub fn headers(&self) -> &HeaderMap {
        self.request.headers()
    }

    pub fn insert<T>(&mut self, value: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        self.request.extensions_mut().insert(value);
    }

    /// Returns a reference to the HTTP method associated with the request.
    pub fn method(&self) -> &Method {
        self.request.method()
    }

    pub fn param<'a>(&'a self, name: &'a str) -> PathParam<'a> {
        let value = self
            .path_params
            .get(name)
            .copied()
            .map(|(start, end)| &self.uri().path()[start..end]);

        PathParam::new(name, value)
    }

    pub fn query(&self) -> QueryParams {
        QueryParams::new(&self.query_params, self.uri().query().unwrap_or(""))
    }

    /// Returns a reference to the uri associated with the request.
    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }

    /// Returns the HTTP version associated with the request.
    pub fn version(&self) -> Version {
        self.request.version()
    }
}
