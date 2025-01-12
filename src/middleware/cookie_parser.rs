use cookie::{Cookie, SplitCookies};
use http::header::COOKIE;
use std::marker::PhantomData;

use crate::middleware::{BoxFuture, Middleware, Next};
use crate::{Error, Request, Response};

/// Defines how to parse cookies from a cookie string.
///
pub trait ParseCookies {
    type Iter: Iterator<Item = Result<Cookie<'static>, Self::Error>> + Send + 'static;
    type Error: Into<Error> + Send;

    fn parse_cookies(input: String) -> Self::Iter;
}

/// Decodes percent-encoded cookie strings with the `percent-encoded` crate
/// before they are parsed.
///
pub struct ParseEncoded;

/// Parses cookie strings without decoding them.
///
pub struct ParseUnencoded;

/// Middleware that parses request cookies and set's the response `Cookie`
/// header.
///
pub struct CookieParser<T = ParseEncoded> {
    _parse: PhantomData<T>,
}

impl CookieParser {
    pub fn new() -> Self {
        Self {
            _parse: PhantomData,
        }
    }
}

impl Default for CookieParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CookieParser<ParseUnencoded> {
    pub fn unencoded() -> Self {
        Self {
            _parse: PhantomData,
        }
    }
}

impl<State, T> Middleware<State> for CookieParser<T>
where
    State: Send + Sync + 'static,
    T: ParseCookies + Send + Sync,
{
    fn call(
        &self,
        mut request: Request<State>,
        next: Next<State>,
    ) -> BoxFuture<Result<Response, Error>> {
        // Attempt to parse the value of the `Cookie` header if it exists and
        // contains a valid cookie string.
        //
        let existing = request
            .header(COOKIE)
            .map(|c| c.to_str().map(|s| s.to_owned()))
            .and_then(|to_str_result| match to_str_result {
                // The cookie header contains a valid cookie string. Parse it.
                Ok(input) => {
                    let mut cookies = vec![];

                    T::parse_cookies(input).for_each(|result| match result {
                        Ok(cookie) => cookies.push(cookie),
                        Err(error) => {
                            // Placeholder for tracing...
                            let _ = error;
                        }
                    });

                    if !cookies.is_empty() {
                        Some(cookies)
                    } else {
                        None
                    }
                }

                // The cookie header contains characters that are not visible ASCII.
                Err(error) => {
                    let _ = error; // Placeholder for tracing...
                    None
                }
            });

        if let Some(cookies) = &existing {
            let jar = request.cookies_mut();
            for cookie in cookies {
                jar.add_original(cookie.clone());
            }
        }

        Box::pin(async {
            // Call the next middleware to get a response.
            let mut response = next.call(request).await?;

            if let Some(cookies) = existing {
                let jar = response.cookies_mut();
                for cookie in cookies {
                    jar.add_original(cookie);
                }
            }

            // Return the response.
            Ok(response)
        })
    }
}

impl ParseCookies for ParseEncoded {
    type Iter = SplitCookies<'static>;
    type Error = cookie::ParseError;

    fn parse_cookies(input: String) -> Self::Iter {
        Cookie::split_parse_encoded(input)
    }
}

impl ParseCookies for ParseUnencoded {
    type Iter = SplitCookies<'static>;
    type Error = cookie::ParseError;

    fn parse_cookies(input: String) -> Self::Iter {
        Cookie::split_parse(input)
    }
}
