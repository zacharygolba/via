use cookie::Cookie;
use http::HeaderValue;
use http::header::{COOKIE, SET_COOKIE};

use crate::middleware::{BoxFuture, Middleware};
use crate::request::{Request, RequestHead};
use crate::util::UriEncoding;
use crate::{Error, Next, raise};

#[derive(Debug)]
pub struct Cookies {
    codec: UriEncoding,
}

fn encode_set_cookie_header(codec: &UriEncoding, cookie: &Cookie) -> Result<HeaderValue, Error> {
    Ok(match codec {
        UriEncoding::Percent => cookie.encoded().to_string().try_into()?,
        UriEncoding::Unencoded => cookie.to_string().try_into()?,
    })
}

fn parse_cookie_header<State>(
    request: &mut Request<State>,
    codec: &UriEncoding,
) -> Result<Vec<Cookie<'static>>, Error> {
    let mut results = {
        let Some(input) = request.header(COOKIE)? else {
            return Ok(vec![]);
        };

        match codec {
            UriEncoding::Percent => Cookie::split_parse_encoded(input.to_owned()),
            UriEncoding::Unencoded => Cookie::split_parse(input.to_owned()),
        }
    };

    let RequestHead { cookies, .. } = request.head_mut();
    let mut existing = Vec::new();

    for result in &mut results {
        match result {
            Ok(cookie) => {
                existing.push(cookie.clone());
                cookies.add_original(cookie);
            }
            Err(error) => {
                return Err(raise!(400, error));
            }
        }
    }

    Ok(existing)
}

impl Cookies {
    fn new(codec: UriEncoding) -> Self {
        Self { codec }
    }

    pub fn percent_decode() -> Self {
        Self::new(UriEncoding::Percent)
    }

    pub fn unencoded() -> Self {
        Self::new(UriEncoding::Unencoded)
    }
}

impl<State> Middleware<State> for Cookies
where
    State: Send + Sync + 'static,
{
    fn call(&self, mut request: Request<State>, next: Next<State>) -> BoxFuture {
        let Self { codec } = *self;
        let result = parse_cookie_header(&mut request, &codec);

        Box::pin(async move {
            let existing = result?;
            let mut response = next.call(request).await?;
            let (cookies, headers) = response.cookies_and_headers_mut();

            for cookie in existing {
                cookies.add_original(cookie);
            }

            for cookie in cookies.delta() {
                let set_cookie = encode_set_cookie_header(&codec, cookie)?;
                headers.try_append(SET_COOKIE, set_cookie)?;
            }

            Ok(response)
        })
    }
}
