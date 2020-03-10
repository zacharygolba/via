use crate::{http::Extensions, routing::Router, App, Body, Response};
use futures::future::{ready, BoxFuture, FutureExt, Map, Ready};
use hyper::service::Service as HyperService;
use std::{convert::Infallible, ops::Deref, sync::Arc, task::*};

type Request = crate::http::Request<hyper::Body>;
type Result<T = Response, E = Infallible> = std::result::Result<T, E>;

pub struct MakeService(Service);

pub struct Service(Arc<Value>);

pub struct Value {
    router: Router,
    state: Arc<Extensions>,
}

impl From<Service> for MakeService {
    #[inline]
    fn from(service: Service) -> MakeService {
        MakeService(service)
    }
}

impl From<App> for MakeService {
    #[inline]
    fn from(app: App) -> MakeService {
        Service::from(app).into()
    }
}

impl<T> HyperService<T> for MakeService {
    type Error = Infallible;
    type Future = Ready<Result<Self::Response>>;
    type Response = Service;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, _: T) -> Self::Future {
        let MakeService(service) = self;
        ready(Ok(service.clone()))
    }
}

impl Clone for Service {
    #[inline]
    fn clone(&self) -> Service {
        Service(Arc::clone(&self.0))
    }
}

impl Deref for Service {
    type Target = Value;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<App> for Service {
    #[inline]
    fn from(app: App) -> Service {
        let App { router, state } = app;
        let state = Arc::new(state);

        Service(Arc::new(Value { router, state }))
    }
}

impl HyperService<Request> for Service {
    type Error = Infallible;
    type Future = Map<BoxFuture<'static, crate::Result>, fn(crate::Result) -> Result>;
    type Response = Response;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, request: Request) -> Self::Future {
        let request = request.map(Body);
        let state = self.state.clone();

        self.router
            .visit((state, request).into())
            .map(|result| match result {
                Ok(response) => Ok(response),
                Err(error) => Ok(error.into()),
            })
    }
}
