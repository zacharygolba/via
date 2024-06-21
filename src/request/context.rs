use http::{HeaderMap, Method, Request, Uri, Version};
use hyper::body::Incoming;
use smallvec::SmallVec;

use super::{
    path_param::PathParams,
    query_param::{QueryParamValues, QueryParams},
    query_parser::parse_query_params,
    Body, PathParam,
};
use crate::{Error, Result};

pub(crate) type IncomingRequest = Request<Incoming>;

#[derive(Debug)]
pub struct Context {
    request: Request<Body>,
    path_params: PathParams,
    query_params: Option<QueryParams>,
}

impl Context {
    pub(crate) fn new(request: IncomingRequest, path_params: PathParams) -> Self {
        Context {
            request: request.map(|body| Body { inner: Some(body) }),
            query_params: None,
            path_params,
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

    pub fn param<'a>(&self, name: &'a str) -> PathParam<'_, 'a> {
        let path = self.request.uri().path();

        for (param, range) in &self.path_params {
            if name == *param {
                return PathParam::new(name, path, Some(range));
            }
        }

        PathParam::new(name, path, None)
    }

    pub fn query<'a>(&mut self, name: &'a str) -> QueryParamValues<'_, 'a> {
        let mut values = SmallVec::new();

        let query = self.request.uri().query().unwrap_or("");
        let params = self
            .query_params
            .get_or_insert_with(|| parse_query_params(query));

        for (param, range) in params.iter() {
            if name == param {
                values.push(range);
            }
        }

        QueryParamValues::new(name, query, values)
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
