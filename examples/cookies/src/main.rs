use cookie::{Cookie, Key};
use std::process::ExitCode;
use via::middleware::{cookie_parser, error_boundary};
use via::{Next, Request, Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

/// A struct used to store application state.
///
struct State {
    /// The secret key used to sign, verify, and optionally encrypt cookies. The
    /// value of this key should be kept secret and changed periodically.
    ///
    secret: Key,
}

/// Responds with a greeting message with the name provided in the request uri
/// path.
///
async fn hello(request: Request<State>, _: Next<State>) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

/// Increments the value of the "n_visits" counter to the console. Returns a
/// response with a message confirming the operation was successful.
///
async fn count_visits(request: Request<State>, next: Next<State>) -> via::Result {
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
    let mut counter = request
        .cookies()
        .and_then(|jar| jar.signed(secret).get("n_visits"))
        .and_then(|cookie| cookie.value().parse().ok())
        .unwrap_or(0i32);

    // Call the next middleware to get the response.
    let mut response = next.call(request).await?;

    // Print the number of times the user has visited the site to stdout.
    println!("User has visited {} times.", counter);

    // If the response status is not successful, return early without updating
    // the "n_visits" cookie.
    //
    if !response.status().is_success() {
        return Ok(response);
    }

    // Increment the visit counter.
    counter += 1;

    // Create a new cookie with the updated value. Set the path to / so it is
    // available on every route.
    let cookie = Cookie::build(Cookie::new("n_visits", counter.to_string()))
        .path("/")
        .build();

    // Add the updated "n_visits" cookie to the response cookies.
    response.cookies_mut().signed_mut(secret).add(cookie);

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
async fn main() -> Result<ExitCode, Error> {
    // Load the environment variables from the ".env" file. This is where we
    // keep the secret key in development. In production, you may want to
    // configure the secret key using a different method. For example, using
    // a parameter store or secret manager to set the value in the environment
    // at the time of deployment.
    //
    dotenvy::dotenv().ok();

    // Create a new app by calling the `via::app` function.
    let mut app = via::app(State {
        secret: get_secret_from_env(),
    });

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::map(|error| {
        eprintln!("error: {}", error);
        error.use_canonical_reason()
    }));

    // The CookieParser middleware can be added at any depth of the route tree.
    // In this example, we add it to the root of the app. This means that every
    // request will pass through the CookieParser middleware.
    //
    app.include(cookie_parser::parse_encoded());

    // Add the count_visits middleware to the app at "/".
    app.include(count_visits);

    // Add a route that responds with a greeting message.
    app.at("/hello/:name").respond(via::get(hello));

    Ok(via::start(app).listen(("127.0.0.1", 8080)).await?)
}
