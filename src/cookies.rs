use cookie::{Cookie, ParseError};
use http::header::{self, COOKIE, SET_COOKIE};
use std::fmt::{self, Display, Formatter};

use crate::Next;
use crate::middleware::{BoxFuture, Middleware};
use crate::request::{Request, RequestHead};
use crate::util::UriEncoding;

/// An error occurred while writing a Set-Cookie header to a response.
///
#[derive(Debug)]
struct SetCookieError;

/// Parse request cookies and serialize response cookies.
///
/// A bidirectional middleware that parses the cookie header of an incoming
/// request and extends the request's cookie jar with the extracted cookies,
/// then calls `next` to obtain a response and serializes any modified cookies
/// into `Set-Cookie` headers.
///
/// # Example
///
/// ```no_run
/// use cookie::{Cookie, SameSite};
/// use std::process::ExitCode;
/// use via::{App, Cookies, Error, Next, Request, Response, Server};
///
/// async fn greet(request: Request, _: Next) -> via::Result {
///     use time::Duration;
///
///     // `should_set_name` indicates whether "name" was sourced from the
///     // request URI. When false, the "name" cookie should not be modified.
///     //
///     // `name` is a Cow that contains either the percent-decoded value of
///     // the "name" cookie or the percent-decoded value of the "name"
///     // parameter in the request URI.
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
///         response.cookies_mut().add(
///             Cookie::build(("name", name.into_owned()))
///                 .http_only(true)
///                 .max_age(Duration::hours(1))
///                 .path("/")
///                 .same_site(SameSite::Strict)
///                 .secure(true),
///         );
///     }
///
///     Ok(response)
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<ExitCode, Error> {
///     let mut app = App::new(());
///
///     // Provide cookie support for downstream middleware.
///     app.middleware(Cookies::new().allow("name").percent_decode());
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
/// The Cookies middleware responds with a `500` error if any of the following
/// conditions are met:
///
/// - A Set-Cookie header cannot be constructed
/// - The maximum capacity of the response header map is exceeded
///
/// # Security
///
/// In production, we recommend using either a
/// [`SignedJar`](https://docs.rs/cookie/latest/cookie/struct.SignedJar.html)
/// or
/// [`PrivateJar`](https://docs.rs/cookie/latest/cookie/struct.PrivateJar.html)
/// to store security sensitive cookies.
///
/// A _signed jar_ signs all cookies added to it and verifies cookies retrieved
/// from it, preventing clients from tampering with or fabricating cookie data.
/// A _private jar_ both signs and encrypts cookies, providing all the
/// guarantees of a signed jar while also ensuring confidentiality.
///
/// ## Best Practices
///
/// As a best practice, in order to mitigate the vast majority of security
/// related concerns of shared state with a client via cookies–we recommend
/// setting `HttpOnly`, `Max-Age`, `SameSite=Strict`, and `Secure` for every
/// cookie used by your application.
///
/// - `HttpOnly`<br>
///   Prevents client-side scripts from accessing the cookie, mitigating cross-
///   site scripting (XSS) attacks. This should be enabled for any cookie that
///   does not need to be accessed directly from JavaScript. Requests made from
///   JavaScript using the Fetch API with `credentials: "include"` or
///   `"same-origin"` automatically include all relevant cookies for the
///   request's origin, including those marked as `HttpOnly`.
///
/// - `Max-Age`<br>
///   Limits how long the browser will store and send the cookie. This reduces
///   the window in which a leaked or stolen cookie can be used, and helps
///   prevent session accumulation on the client.
///
/// - `SameSite=Strict`<br>
///   Restricts cookies to same-site requests, mitigating CSRF attacks. If the
///   cookie does not need to be shared cross-site, this setting practically
///   eliminates CSRF risk in modern browsers. However, it prevents
///   authentication flows that involve redirects from external identity
///   providers (OAuth, SAML, etc.).
///
/// - `Secure`<br>
///   Instructs the client to only include the cookie in requests made using
///   the `https:` scheme or to `localhost`.
///
/// ```no_run
/// use cookie::{Cookie, Key, SameSite};
/// use http::StatusCode;
/// use serde::Deserialize;
/// use std::process::ExitCode;
/// use via::{App, Cookies, Error, Next, Payload, Request, Response, Server};
///
/// #[derive(Deserialize)]
/// struct Login {
///     username: String,
///     password: String,
/// }
///
/// struct Unicorn {
///     secret: Key,
/// }
///
/// async fn login(request: Request<Unicorn>, _: Next<Unicorn>) -> via::Result {
///     use time::Duration;
///
///     let (head, body) = request.into_parts();
///     let state = head.into_state();
///
///     let Login { username, password } = body.into_future().await?.parse_json()?;
///
///     // Insert username and password verification here...
///     // For now, we'll just assert that the password is not empty.
///     if password.is_empty() {
///         via::raise!(401, message = "Invalid username or password.");
///     }
///
///     // Generate a response with no content.
///     //
///     // If we were verifying that a user with the provided username and
///     // password exists in a database table, we'd probably respond with the
///     // matching row as JSON.
///     let mut response = Response::build().status(StatusCode::NO_CONTENT).finish()?;
///
///     // Add our session cookie that contains the username of the active user
///     // to our private cookie jar. The value of the cookie will be signed
///     // and encrypted before it is included as a set-cookie header.
///     response.cookies_mut().private_mut(&state.secret).add(
///         Cookie::build(("unicorn-session", username))
///             .http_only(true)
///             .max_age(Duration::hours(1))
///             .path("/")
///             .same_site(SameSite::Strict)
///             .secure(true),
///     );
///
///     Ok(response)
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<ExitCode, Error> {
///     let mut app = App::new(Unicorn {
///         secret: std::env::var("VIA_SECRET_KEY")
///             .map(|secret| secret.as_bytes().try_into())
///             .expect("missing required env var: VIA_SECRET_KEY")
///             .expect("unexpected end of input while parsing VIA_SECRET_KEY"),
///     });
///
///     // Unencoded cookie support.
///     app.middleware(Cookies::new().allow("unicorn-session"));
///
///     // Add our login route to our application.
///     app.route("/auth/login").respond(via::post(login));
///
///     // Start serving our application from http://localhost:8080/.
///     Server::new(app).listen(("127.0.0.1", 8080)).await
/// }
/// ```
///
pub struct Cookies {
    encoding: UriEncoding,
    allow: Vec<String>,
}

fn encode_set_cookie_header(
    encoding: &UriEncoding,
    cookie: &Cookie,
) -> Result<http::HeaderValue, SetCookieError> {
    let encoded = match encoding {
        UriEncoding::Percent => cookie.encoded().to_string(),
        _ => cookie.to_string(),
    };

    Ok(encoded.try_into()?)
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

    /// Add the provided cookie name to the allow list.
    ///
    /// By default, the Cookies middleware ignores cookies with names that are
    /// not explicitly allowed. This filters out irrelevant cookies and keeps
    /// the number of cookies in the request and response cookie jars bounded.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Cookies};
    /// # let mut app = App::new(());
    /// app.middleware(Cookies::new().allow("via-session"));
    /// ```
    ///
    pub fn allow(mut self, name: impl AsRef<str>) -> Self {
        self.allow.push(name.as_ref().to_owned());
        self
    }

    /// Specify that cookies should be percent-decoded when parsed and percent-
    /// encoded when serialized as a Set-Cookie header.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Cookies};
    /// # let mut app = App::new(());
    /// app.middleware(Cookies::new().allow("via:session").percent_decode());
    /// ```
    ///
    pub fn percent_decode(mut self) -> Self {
        self.encoding = UriEncoding::Percent;
        self
    }

    fn parse<'a>(&self, input: &'a str) -> impl Iterator<Item = Result<Cookie<'a>, ParseError>> {
        let Self { encoding, allow } = self;
        let results = match encoding {
            UriEncoding::Percent => Cookie::split_parse_encoded(input),
            _ => Cookie::split_parse(input),
        };

        results.filter(|result| match result {
            Ok(cookie) => allow.iter().any(|name| name.as_str() == cookie.name()),
            Err(_) => false,
        })
    }
}

impl Default for Cookies {
    fn default() -> Self {
        Self {
            encoding: UriEncoding::Unencoded,
            allow: vec![],
        }
    }
}

impl<State> Middleware<State> for Cookies
where
    State: Send + Sync + 'static,
{
    fn call(&self, mut request: Request<State>, next: Next<State>) -> BoxFuture {
        let RequestHead { cookies, parts, .. } = request.head_mut();
        let mut existing = Vec::with_capacity(self.allow.len());

        if let Some(header) = parts.headers.get(COOKIE)
            && let Ok(input) = header.to_str()
        {
            for result in self.parse(input) {
                let original = match result {
                    Ok(cookie) => cookie.into_owned(),
                    Err(error) => {
                        // Placeholder for tracing...
                        if cfg!(debug_assertions) {
                            eprintln!("warn: {}", error);
                        }

                        continue;
                    }
                };

                existing.push(original.clone());
                cookies.add_original(original);
            }
        }

        let future = next.call(request);
        let Self { encoding, .. } = *self;

        Box::pin(async move {
            let mut response = future.await?;
            let (cookies, headers) = response.cookies_and_headers_mut();

            for cookie in existing {
                cookies.add_original(cookie);
            }

            cookies.delta().try_for_each(|cookie| {
                let set_cookie = encode_set_cookie_header(&encoding, cookie)?;
                headers.try_append(SET_COOKIE, set_cookie)?;
                Ok::<_, SetCookieError>(())
            })?;

            Ok(response)
        })
    }
}

impl std::error::Error for SetCookieError {}

impl Display for SetCookieError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "An error occurred while writing a Set-Cookie header to a response."
        )
    }
}

impl From<header::MaxSizeReached> for SetCookieError {
    fn from(_: header::MaxSizeReached) -> Self {
        Self
    }
}

impl From<header::InvalidHeaderValue> for SetCookieError {
    fn from(_: header::InvalidHeaderValue) -> Self {
        Self
    }
}
