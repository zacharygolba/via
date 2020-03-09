use crate::{App, Body, Error, Response};
use futures::future::{ready, FutureExt, Map, Ready};
use hyper::service::Service as HyperService;
use std::{convert::Infallible, sync::Arc, task::*};

type Request = crate::http::Request<hyper::Body>;
type Result<T = Response, E = Infallible> = std::result::Result<T, E>;

pub struct MakeService {
    service: Service,
}

pub struct Service {
    app: Arc<App>,
}

impl From<Service> for MakeService {
    #[inline]
    fn from(service: Service) -> MakeService {
        MakeService { service }
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
        ready(Ok(self.service.clone()))
    }
}

impl Clone for Service {
    #[inline]
    fn clone(&self) -> Service {
        Service {
            app: Arc::clone(&self.app),
        }
    }
}

impl From<App> for Service {
    #[inline]
    fn from(app: App) -> Service {
        Service { app: Arc::new(app) }
    }
}

impl HyperService<Request> for Service {
    type Error = Infallible;
    type Future = Map<crate::Future, fn(Result<Response, Error>) -> Result>;
    type Response = Response;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, request: Request) -> Self::Future {
        let request = request.map(Body);

        self.app.call(request).map(|result| match result {
            Ok(response) => Ok(response),
            Err(error) => Ok(error.into()),
        })
    }
}
