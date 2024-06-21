use http::{HeaderMap, Method, Request, Uri, Version};

use super::{
    path_param::PathParams,
    query_param::QueryParamValues,
    query_parser::{parse_query_params, QueryParams},
    Body, PathParam,
};
use crate::{Error, Result};

pub(crate) type IncomingRequest = Request<hyper::body::Incoming>;

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

    pub fn param<'a>(&'a self, name: &'a str) -> PathParam<'a> {
        let value = self
            .path_params
            .get(name)
            .copied()
            .map(|(start, end)| &self.uri().path()[start..end]);

        PathParam::new(name, value)
    }

    pub fn query<'a, 'b>(&'a mut self, name: &'b str) -> QueryParamValues<'a, 'b> {
        let query = self.request.uri().query().unwrap_or_default();
        let values = self
            .query_params
            .get_or_insert_with(|| parse_query_params(query))
            .iter()
            .filter_map(|(param, range)| if name == param { Some(*range) } else { None })
            .collect();

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
