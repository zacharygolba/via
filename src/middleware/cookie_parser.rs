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
        //
        let (parser_output, request_cookies) = match cookie_header.map(|value| value.to_str()) {
            // There are no cookies to parse.
            None => {
                return next.call(request);
            }

            // The cookie header contains characters that are not visible ASCII.
            Some(Err(error)) => {
                let _ = error; // Placeholder for tracing...
                return next.call(request);
            }

            // The cookie header contains a valid cookie string. Parse it.
            Some(Ok(cookie_str)) => {
                // Get an owned String from cookie_str.
                let input = cookie_str.to_string();

                // Parse the cookie string into an iter of results.
                let mut output = T::parse_cookies(input).peekable();

                // Get a mutable reference to the request cookies.
                let cookies = if output.peek().is_some() {
                    // The parser parsed some cookies. Allocate a new
                    // `Box<CookieJar>` and get a mutable reference to it.
                    request.cookies_mut()
                } else {
                    // The parser did not parse any cookies.
                    return next.call(request);
                };

                (output, cookies)
            }
        };

        // Iterate over the remaining results in the parser output. If a cookie
        // was able to be parsed without error, add it to request_cookies.
        //
        parser_output.for_each(|result| match result {
            Ok(cookie) => request_cookies.add_original(cookie),
            Err(error) => {
                let _ = error; // Placeholder for tracing...
            }
        });

        // Clone request_cookies so we can merge them with the response cookies
        // when they become available. This is necessary because we need to pass
        // ownership of the request to the next middleware.
        //
        let mut merged_cookies = Box::new(request_cookies.clone());

        Box::pin(async {
            // Call the next middleware to get a response.
            let mut response = next.call(request).await?;

            if let Some(cookies) = response.cookies().map(|jar| jar.iter()) {
                cookies.cloned().for_each(|cookie| {
                    merged_cookies.add(cookie);
                });

                // Replace the response cookies with merged_cookies. The delta
                // will be calculated and converted into Set-Cookie headers
                // before the response is sent to the client.
                //
                response.set_cookies(merged_cookies);
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
