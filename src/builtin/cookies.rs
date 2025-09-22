use cookie::{Cookie, SplitCookies};
use http::header::COOKIE;

use crate::{BoxFuture, Error, Middleware, Next, Request};

#[derive(Debug)]
pub struct CookieParser(Codec);

#[derive(Clone, Copy, Debug)]
enum Codec {
    PercentEncoded,
    Unencoded,
}

pub fn percent_decode() -> CookieParser {
    CookieParser(Codec::PercentEncoded)
}

pub fn unencoded() -> CookieParser {
    CookieParser(Codec::Unencoded)
}

impl CookieParser {
    fn parse(&self, input: String) -> SplitCookies<'static> {
        match self.0 {
            Codec::PercentEncoded => Cookie::split_parse_encoded(input),
            Codec::Unencoded => Cookie::split_parse(input),
        }
    }
}

impl<State> Middleware<State> for CookieParser
where
    State: Send + Sync + 'static,
{
    fn call(&self, mut request: Request<State>, next: Next<State>) -> BoxFuture {
        let Self(codec) = *self;
        let parse_result = 'parse: {
            let mut existing = Vec::new();
            let input = match request.header(COOKIE) {
                Ok(Some(str)) => str.to_owned(),
                Err(error) => break 'parse Err(error),
                Ok(None) => break 'parse Ok(existing),
            };

            let jar = request.cookies_mut();

            for result in self.parse(input) {
                match result {
                    Err(error) => break 'parse Err(Error::bad_request(error)),
                    Ok(cookie) => {
                        existing.push(cookie.clone());
                        jar.add_original(cookie);
                    }
                }
            }

            Ok(existing)
        };

        Box::pin(async move {
            let existing = parse_result?;
            let mut response = next.call(request).await?;

            let jar = response.cookies_mut();
            for cookie in existing {
                jar.add_original(cookie);
            }

            response.set_cookies(|cookie| match codec {
                Codec::PercentEncoded => cookie.encoded().to_string(),
                Codec::Unencoded => cookie.to_string(),
            })?;

            Ok(response)
        })
    }
}
