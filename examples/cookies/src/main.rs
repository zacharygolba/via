use cookie::{Cookie, Key, SameSite};
use std::env;
use std::process::ExitCode;
use via::{App, Cookies, Error, Response, Server};

type Request = via::Request<CookiesExample>;
type Next = via::Next<CookiesExample>;

/// A struct used to store application state.
///
struct CookiesExample {
    /// The secret key used to sign, verify, and optionally encrypt cookies. The
    /// value of this key should be kept secret and changed periodically.
    ///
    secret: Key,
}

/// Increments the value of the "counter" counter to the console. Returns a
/// response with a message confirming the operation was successful.
///
async fn counter(request: Request, next: Next) -> via::Result {
    // Clone the state from the request so we can access the secret key after
    // passing ownership of the request to the next middleware.
    //
    let state = request.state().clone();

    // Get the value of the "counter" cookie from the request before passing
    // ownership of the request to the next middleware. In this example, we are
    // using the signed cookie jar to store and retrieve the "counter" cookie.
    // If
    //
    let value = request
        .cookies()
        .signed(&state.secret)
        .get("counter")
        .map_or(Ok(0i32), |cookie| cookie.value().parse())?;

    // Call the next middleware to get the response.
    let mut response = next.call(request).await?;

    // Print the number of times the user has visited the site to stdout.
    println!("User has visited {} times.", value);

    // If the response status code is in 200..=299, increment the counter.
    if response.status().is_success() {
        let incr = (value + 1).to_string();
        let jar = response.cookies_mut();

        jar.signed_mut(&state.secret).add(
            Cookie::build(("counter", incr))
                .same_site(SameSite::Strict)
                .http_only(true)
                .path("/"),
        );
    }

    // Return the response.
    Ok(response)
}

/// Responds with a greeting message with the name provided in the request uri
/// path.
///
async fn greet(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    // Load the environment variables from the ".env" file. This is where we
    // keep the secret key in development. In production, you may want to
    // configure the secret key using a different method. For example, using
    // a parameter store or secret manager to set the value in the environment
    // at the time of deployment.
    //
    dotenvy::dotenv()?;

    // Create a new application.
    let mut app = App::new(CookiesExample {
        secret: env::var("VIA_SECRET_KEY")
            .map(|secret| Key::from(secret.as_bytes()))
            .expect("missing required env var: VIA_SECRET_KEY"),
    });

    // The CookieParser middleware can be added at any depth of the route tree.
    // In this example, we add it to the root of the app. This means that every
    // request will pass through the CookieParser middleware.
    app.middleware(Cookies::new());

    // Add the count_visits middleware to the app at "/".
    app.middleware(counter);

    // Add a route that responds with a greeting message.
    app.route("/hello/:name").respond(via::get(greet));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
