use http::{HeaderMap, Method, Uri, Version};

use super::{Body, HyperRequest, PathParam, PathParams};
use crate::Result;

#[derive(Debug)]
pub struct Context {
    request: http::Request<Body>,
    params: PathParams,
}

impl Context {
    pub(crate) fn new(request: HyperRequest, params: PathParams) -> Self {
        Context {
            request: request.map(Body::new),
            params,
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
        match self.request.extensions().get() {
            Some(value) => Ok(value),
            None => crate::bail!("unknown type"),
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
            .params
            .get(name)
            .copied()
            .map(|(start, end)| &self.uri().path()[start..end]);

        PathParam::new(name, value)
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
