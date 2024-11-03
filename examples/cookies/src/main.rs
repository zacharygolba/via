use cookie::{Cookie, Key};
use std::process::ExitCode;
use via::middleware::CookieParser;
use via::{BoxError, Response, Server};

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

/// Responds with a greeting message with the name provided in the request uri
/// path.
///
async fn hello(request: Request, _: Next) -> via::Result<String> {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Ok(format!("Hello, {}!", name))
}

/// Increments the value of the "n_visits" counter to the console. Returns a
/// response with a message confirming the operation was successful.
///
async fn count_visits(request: Request, next: Next) -> via::Result<Response> {
    // Clone the state from the request so we can access the secret key after
    // passing ownership of the request to the next middleware.
    //
    let state = request.state().clone();

    // Get a reference to the secret key from state.
    let secret = &state.secret;

    // Get the value of the "n_visits" cookie from the request before passing
    // ownership of the request to the next middleware. In this example, we are
    // using the signed cookie jar to store and retrieve the "n_visits" cookie.
    // If
    //
    let mut n_visits: i32 = match request.cookies().signed(secret).get("n_visits") {
        Some(cookie) => cookie.value().parse().unwrap_or(0),
        None => 0,
    };

    // Call the next middleware to get the response.
    let mut response = next.call(request).await?;

    // If the response status is not successful, return early without updating
    // the "n_visits" cookie.
    //
    if !response.status().is_success() {
        return Ok(response);
    }

    // Increment the value of the "n_visits" counter.
    n_visits += 1;

    // Get a mutable reference to the response cookies.
    let mut cookies = response.cookies_mut().signed_mut(secret);

    // Create a new cookie with the updated "n_visits" value.
    let mut cookie = Cookie::new("n_visits", n_visits.to_string());

    // Set the cookie's path to "/" so it's available to all routes.
    cookie.set_path("/");

    // Print the number of times the user has visited the site to stdout.
    println!("User has visited {} times.", n_visits);

    // Add the updated "n_visits" cookie to the response cookies.
    cookies.add(cookie);

    // Return the response.
    Ok(response)
}

/// Load the secret key from the "VIA_SECRET_KEY" environment variable.
///
fn get_secret_from_env() -> Key {
    std::env::var("VIA_SECRET_KEY")
        .map(|secret| Key::from(secret.as_bytes()))
        .expect("missing required env var: VIA_SECRET_KEY")
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    // Load the environment variables from the ".env" file. This is where we
    // keep the secret key in development. In production, you may want to
    // configure the secret key using a different method. For example, using
    // a parameter store or secret manager to set the value in the environment
    // at the time of deployment.
    //
    dotenvy::dotenv().ok();

    // Create a new app by calling the `via::app` function.
    let mut app = via::new(CookiesExample {
        secret: get_secret_from_env(),
    });

    // The CookieParser middleware can be added at any depth of the route tree.
    // In this example, we add it to the root of the app. This means that every
    // request will pass through the CookieParser middleware.
    //
    app.include(CookieParser::new());

    // Add the count_visits middleware to the app at "/".
    app.include(count_visits);

    // Add a route that responds with a greeting message.
    app.at("/hello/:name").respond(via::get(hello));

    // Start the server.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
