use cookie::Cookie;
use http::HeaderValue;
use http::header::{COOKIE, SET_COOKIE};

use crate::middleware::{BoxFuture, Middleware};
use crate::request::{Request, RequestHead};
use crate::util::UriEncoding;
use crate::{Error, Next, err};

/// Parse request cookies and serialize response cookies.
///
/// A bidirectional middleware that parses the cookie header of an incoming
/// request, extends the request cookie jar with the cookies extracted from the
/// cookie header. Then, calls `next` to get a response and serializes any
/// cookies that changed into set-cookie headers.
///
/// # Example
///
/// ```no_run
/// use cookie::{Cookie, SameSite};
/// use std::process::ExitCode;
/// use via::{App, Cookies, Error, Next, Request, Response, Server};
///
/// async fn greet(request: Request, _: Next) -> via::Result {
///     // The bool is a flag indicating whether or not "name" was sourced from
///     // the request uri. When false, do not set the "name" cookie.
///     //
///     // The Cow contains either the percent-decoded value of the "name"
///     // cookie or the percent-decoded value of the "name" parameter in the
///     // request uri.
///     let (should_set_name, name) = match request.cookies().get("name") {
///         Some(cookie) => (false, cookie.value().into()),
///         None => (true, request.param("name").percent_decode().into_result()?),
///     };
///
///     // Build the greeting response using a reference to name.
///     let mut response = Response::build().text(format!("Hello, {}!", name.as_ref()))?;
///
///     // If "name" came from the request uri, set the "name" cookie.
///     if should_set_name {
///         let saved_name = Cookie::build(("name", name.into_owned()))
///             .same_site(SameSite::Strict)
///             .http_only(true)
///             .path("/");
///
///         response.cookies_mut().add(saved_name);
///     }
///
///     Ok(response)
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<ExitCode, Error> {
///     let mut app = App::new(());
///
///     // Provides cookie support for downstream middleware.
///     app.middleware(Cookies::percent_decode());
///
///     // Respond with a greeting when a user visits /hello/:name.
///     app.route("/hello/:name").respond(via::get(greet));
///
///     // Start serving our application from http://localhost:8080/.
///     Server::new(app).listen(("127.0.0.1", 8080)).await
/// }
/// ```
///
/// # Errors
///
/// - `400` The cookie header cannot be parsed.
/// - `500` A set-cookie header cannot be constructed or appended to the
///   response.
///
///
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
    let results = {
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

    for result in results {
        let cookie = result.map_err(|error| err!(400, error))?;
        existing.push(cookie.clone());
        cookies.add_original(cookie);
    }

    Ok(existing)
}

impl Cookies {
    /// Returns middleware that provides support for unencoded request and
    /// response cookies.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Cookies};
    /// # let mut app = App::new(());
    /// app.middleware(Cookies::new());
    /// ```
    ///
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns middleware that provides support for `percent%20encoded`
    /// request and response cookies.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Cookies};
    /// # let mut app = App::new(());
    /// app.middleware(Cookies::percent_decode());
    /// ```
    ///
    pub fn percent_decode() -> Self {
        Self {
            codec: UriEncoding::Percent,
        }
    }
}

impl Default for Cookies {
    fn default() -> Self {
        Self {
            codec: UriEncoding::Unencoded,
        }
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
