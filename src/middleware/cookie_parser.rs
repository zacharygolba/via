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
        // Get the value of the `Cookie` header from the request.
        let cookie_header = request.headers().get(COOKIE);

        // Attempt to parse the value of the `Cookie` header if it exists and
        // contains a valid cookie string.
        let parser_output = match cookie_header.map(|value| value.to_str()) {
            None => {
                // There are no cookies to parse.
                return next.call(request);
            }
            Some(Err(error)) => {
                let _ = error;
                // Placeholder for tracing...
                return next.call(request);
            }
            Some(Ok(cookie_str)) => {
                let input = cookie_str.to_string();
                T::parse_cookies(input)
            }
        };

        Box::pin(async {
            // Get a mutable reference to the request cookies.
            let request_cookies = request.cookies_mut();

            // Iterate over each result in the parser output. If the cookie was
            // able to be parsed without error, add it to the request cookies.
            parser_output.for_each(|result| match result {
                Ok(cookie) => request_cookies.add_original(cookie),
                Err(error) => {
                    let _ = error;
                    // Placeholder for tracing...
                }
            });

            // Clone the request cookies so we can merge them with the response
            // cookies when they are available. This is necessary because we need
            // to pass ownership of the request to the next middleware.
            //
            let mut merged_cookies = request_cookies.clone();

            // Call the next middleware to get a response.
            let mut response = next.call(request).await?;

            // Merge the response cookies with our copy of the request cookies at
            // `merged_cookies`. We'll replace the value of response cookies with
            // the combined CookieJar containing both the request and response
            // cookies. We do this to ensure the delta is calculated correctly
            // when the response cookies are serialized into Set-Cookie headers.
            //
            response.cookies().iter().cloned().for_each(|cookie| {
                merged_cookies.add(cookie);
            });

            // Replace the response cookies with the merged cookies.
            *response.cookies_mut() = merged_cookies;

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
