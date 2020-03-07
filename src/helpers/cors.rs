use crate::{
    http::header::{self, HeaderMap, HeaderValue},
    Context, Future, Middleware, Next,
};

#[derive(Default)]
pub struct Cors {
    headers: HeaderMap,
}

#[inline]
pub fn cors(builder: impl FnOnce(&mut Cors)) -> impl Middleware {
    let mut middleware = Cors::default();

    builder(&mut middleware);
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
    #[inline]
    fn call<'a>(&'a self, context: Context, next: Next<'a>) -> Future {
        let headers = self.headers.clone();
        let future = next.call(context);

        Box::pin(async {
            let mut response = future.await?;

            response.headers_mut().extend(headers);
            Ok(response)
        })
    }
}
