use crate::{BoxFuture, Context, Middleware, Next, Result};
use http::header::{self, HeaderMap, HeaderValue};

#[derive(Default)]
pub struct Cors {
    headers: HeaderMap,
}

#[inline]
pub fn cors(f: impl FnOnce(&mut Cors)) -> impl Middleware {
    let mut middleware = Cors::default();

    f(&mut middleware);
    middleware
}

impl Cors {
    pub fn origin(&mut self, value: &'static str) -> &mut Self {
        self.headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static(value),
        );
        self
    }
}

impl Middleware for Cors {
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result> {
        let headers = self.headers.clone();

        Box::pin(async move {
            let mut response = next.call(context).await?;

            response.headers_mut().extend(headers);
            Ok(response)
        })
    }
}
