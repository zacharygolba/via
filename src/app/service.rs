use hyper::body::Incoming;
use hyper::service::Service;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use via_router::Error;

use crate::app::App;
use crate::body::{HttpBody, RequestBody, ResponseBody};
use crate::middleware::FutureResponse;
use crate::request::Request;
use crate::Next;

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_body_size: usize,
}

pub struct ServeRequest {
    result: Result<FutureResponse, Error>,
}

impl<T> AppService<T> {
    #[inline(always)]
    pub(crate) fn new(app: Arc<App<T>>, max_body_size: usize) -> Self {
        Self { app, max_body_size }
    }
}

impl<T: Send + Sync> Service<http::Request<Incoming>> for AppService<T> {
    type Error = Error;
    type Future = ServeRequest;
    type Response = http::Response<HttpBody<ResponseBody>>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let Self {
            ref app,
            max_body_size,
        } = *self;

        let mut request = Request::new(
            Arc::clone(&app.state),
            request.map(|body| RequestBody::new(max_body_size, body).into()),
        );

        let mut next = Next::new();

        for binding in app.router.visit(request.uri().path()) {
            let mut params = Some(request.params_mut());

            for match_key in binding.iter() {
                let (pattern, route) = match app.router.get(*match_key.as_either()) {
                    Err(error) => return ServeRequest { result: Err(error) },
                    Ok(found) => found,
                };

                if let Some((once, label)) = params.take().zip(pattern.as_label()) {
                    once.push(label.clone(), binding.range());
                }

                for match_cond in route {
                    if let Some(middleware) = match_key.as_match(match_cond) {
                        next.stack_mut().push_back(Arc::clone(middleware));
                    }
                }
            }
        }

        ServeRequest {
            result: Ok(next.call(request)),
        }
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<HttpBody<ResponseBody>>, Error>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let pending = match &mut self.result {
            Err(error) => return Poll::Ready(Err(error.clone())),
            Ok(future) => future,
        };

        let response = match pending.as_mut().poll(context) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => e.into(),
            Poll::Ready(Ok(response)) => response,
        };

        Poll::Ready(Ok(response.into_inner()))
    }
}
