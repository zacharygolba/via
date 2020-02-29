use crate::{http::Extensions, routing, routing::Router, Application, Error, Response};
use futures::future::{ready, FutureExt, Map, Ready};
use hyper::{service::Service as HyperService, Body};
use std::{convert::Infallible, sync::Arc, task::*};

type Request = crate::http::Request<Body>;
type Result<T = Response, E = Infallible> = std::result::Result<T, E>;

pub struct MakeService {
    service: Service,
}

pub struct Service {
    router: Arc<Router>,
    state: Arc<Extensions>,
}

impl From<Service> for MakeService {
    #[inline]
    fn from(service: Service) -> MakeService {
        MakeService { service }
    }
}

impl From<Application> for MakeService {
    #[inline]
    fn from(application: Application) -> MakeService {
        MakeService {
            service: application.into(),
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
            router: Arc::clone(&self.router),
            state: Arc::clone(&self.state),
        }
    }
}

impl From<Application> for Service {
    #[inline]
    fn from(application: Application) -> Service {
        Service {
            router: Arc::new(application.router),
            state: Arc::new(application.state),
        }
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
        let Service { router, state } = self;
        let context = crate::Context::new(state.clone(), request);

        routing::visit(&router, context).map(|result| match result {
            Ok(response) => Ok(response),
            Err(e) => Ok(e.into()),
        })
    }
}
