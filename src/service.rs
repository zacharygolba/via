use super::{Application, CallFuture, HttpRequest, HttpResponse};
use futures::future::{ready, Ready};
use hyper::service::Service as HyperService;
use std::{
    convert,
    sync::Arc,
    task::{Context, Poll},
};

type Result<T = ()> = crate::Result<T, convert::Infallible>;

pub struct MakeService {
    service: Service,
}

pub struct Service {
    application: Arc<Application>,
}

impl From<Application> for MakeService {
    fn from(application: Application) -> Self {
        MakeService {
            service: Service::from(application),
        }
    }
}

impl<T> HyperService<T> for MakeService {
    type Error = convert::Infallible;
    type Future = Ready<Result<Self::Response>>;
    type Response = Service;

    fn call(&self, _: T) -> Self::Future {
        ready(Ok(self.service.clone()))
    }
}

impl Clone for Service {
    fn clone(&self) -> Self {
        Service {
            application: Arc::clone(&self.application),
        }
    }
}

impl From<Application> for Service {
    fn from(application: Application) -> Self {
        Service {
            application: Arc::new(application),
        }
    }
}

impl HyperService<HttpRequest> for Service {
    type Error = convert::Infallible;
    type Future = CallFuture;
    type Response = HttpResponse;

    fn call(&self, request: HttpRequest) -> Self::Future {
        self.application.call(request)
    }
}
