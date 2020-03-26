use crate::{Application, Body, BoxFuture, Response};
use futures::future::{ready, FutureExt, Map, Ready};
use hyper::service::Service as HyperService;
use std::{
    convert::{Infallible, TryInto},
    sync::Arc,
    task::*,
};

type Request = crate::http::Request<hyper::Body>;
type Result<T = Response, E = Infallible> = std::result::Result<T, E>;

pub struct MakeService {
    service: Service,
}

pub struct Service {
    app: Arc<Application>,
}

impl From<Application> for MakeService {
    #[inline]
    fn from(app: Application) -> MakeService {
        MakeService {
            service: app.into(),
        }
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

impl From<Application> for Service {
    #[inline]
    fn from(app: Application) -> Service {
        Service { app: Arc::new(app) }
    }
}

impl HyperService<Request> for Service {
    type Error = Infallible;
    type Future = Map<BoxFuture<crate::Result>, fn(crate::Result) -> Result>;
    type Response = Response;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, request: Request) -> Self::Future {
        let context = self.app.context(request.map(Body));
        let future = self.app.routes.visit(context);

        future.map(|result| Ok(result.unwrap_or_else(Response::from)))
    }
}
