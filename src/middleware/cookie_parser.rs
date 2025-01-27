use cookie::{Cookie, SplitCookies};
use http::header::COOKIE;

use super::middleware::Middleware;
use super::next::Next;
use crate::{Error, Request};

pub fn parse_encoded<T>() -> impl Middleware<T> {
    cookie_parser::<ParseEncoded, _>()
}

pub fn parse_unencoded<T>() -> impl Middleware<T> {
    cookie_parser::<ParseUnencoded, _>()
}

/// Defines how to parse cookies from a cookie string.
///
trait ParseCookies {
    type Iter: Iterator<Item = Result<Cookie<'static>, Self::Error>> + Send + 'static;
    type Error: Into<Error> + Send;

    fn parse_cookies(input: String) -> Self::Iter;
}

/// Decodes percent-encoded cookie strings with the `percent-encoded` crate
/// before they are parsed.
///
struct ParseEncoded;

/// Parses cookie strings without decoding them.
///
struct ParseUnencoded;

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

fn cookie_parser<P: ParseCookies, T>() -> impl Middleware<T> {
    move |mut request: Request<T>, next: Next<T>| {
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

                    P::parse_cookies(input).for_each(|result| match result {
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

        // Call the next middleware to get a response.
        let future = next.call(request);

        async {
            // Await the response future.
            let mut response = future.await?;

            if let Some(cookies) = existing {
                let jar = response.cookies_mut();
                for cookie in cookies {
                    jar.add_original(cookie);
                }
            }

            // If any cookies changed during the request, serialize them to
            // Set-Cookie headers and include them in the response headers.
            response.set_cookie_headers();

            // Return the response.
            Ok(response)
        }
    }
}
