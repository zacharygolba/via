use cookie::{Cookie, ParseError, SplitCookies};
use http::header::COOKIE;

use crate::{Error, Middleware, Next, Request};

pub fn parse_encoded<T>() -> impl Middleware<T> {
    cookie_parser::<ParseEncoded, _>()
}

pub fn parse_unencoded<T>() -> impl Middleware<T> {
    cookie_parser::<ParseUnencoded, _>()
}

/// Defines how to parse cookies from a cookie string.
///
trait ParseCookies {
    type Iter: Iterator<Item = Result<Cookie<'static>, ParseError>> + Send + 'static;
    fn parse_cookies(input: String) -> Self::Iter;
}

/// Decodes percent-encoded cookie strings with the `percent-encoded` crate
/// before they are parsed.
///
struct ParseEncoded;

/// Parses cookie strings without decoding them.
///
struct ParseUnencoded;

fn cookie_parser<P: ParseCookies, T>() -> impl Middleware<T> {
    move |mut request: Request<T>, next: Next<T>| {
        let parse_result = 'parse: {
            let mut existing = Vec::new();
            let input = match request.header(COOKIE) {
                Ok(Some(str)) => str.to_owned(),
                Err(error) => break 'parse Err(error),
                Ok(None) => break 'parse Ok(existing),
            };

            let jar = request.cookies_mut();
            for result in P::parse_cookies(input) {
                match result {
                    Ok(cookie) => {
                        existing.push(cookie.clone());
                        jar.add_original(cookie);
                    }
                    Err(error) => {
                        break 'parse Err(Error::bad_request(error));
                    }
                }
            }

            Ok(existing)
        };

        let future = next.call(request);

        async {
            let existing = parse_result?;
            let mut response = future.await?;

            let jar = response.cookies_mut();
            for cookie in existing {
                jar.add_original(cookie);
            }

            response.set_cookies(|cookie| cookie.encoded().to_string())?;

            Ok(response)
        }
    }
}

impl ParseCookies for ParseEncoded {
    type Iter = SplitCookies<'static>;

    fn parse_cookies(input: String) -> Self::Iter {
        Cookie::split_parse_encoded(input)
    }
}

impl ParseCookies for ParseUnencoded {
    type Iter = SplitCookies<'static>;

    fn parse_cookies(input: String) -> Self::Iter {
        Cookie::split_parse(input)
    }
}
