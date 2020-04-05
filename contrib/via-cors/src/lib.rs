use via::{
    http::header::{self, HeaderMap, HeaderValue},
    BoxFuture, Context, Middleware, Next, Result,
};

#[derive(Default)]
pub struct Cors {
    headers: HeaderMap,
}

pub fn cors(configure: impl FnOnce(&mut Cors)) -> Cors {
    let mut middleware = Default::default();

    configure(&mut middleware);
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
