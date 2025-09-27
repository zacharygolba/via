use http::StatusCode;
use std::fmt::Write;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use via::{App, BoxError, Next, Request, Response};

/// A struct of containing the shared state for the application. This struct
/// will be made available to all middleware functions and responders by
/// calling the `state` method on the `Request` struct.
struct Counter {
    /// The number of responses that had a status code in the 1XX-3XX range.
    sucesses: Arc<AtomicU32>,

    /// The number of responses that had a status code in the 4XX-5XX range.
    errors: Arc<AtomicU32>,
}

// Define a helper function to check if a status code is in the 4XX-5XX range.
fn status_is_error(status: StatusCode) -> bool {
    status.is_client_error() || status.is_server_error()
}

/// A middleware function that will either increment the `successes` or
/// `errors` field of the `Counter` state based on the response status.
async fn counter(request: Request<Counter>, next: Next<Counter>) -> via::Result {
    // Clone the `Counter` state by incrementing the reference count of the
    // outer `Arc`. This will allow us to modify the `state` after we pass
    // ownership of `request` to the `next` middleware.
    let state = request.state().clone();

    // Call the next middleware in the app and await the response.
    let response = next.call(request).await?;

    if status_is_error(response.status()) {
        // The status is in the 4XX-5XX range. Increment the `errors` field.
        state.errors.fetch_add(1, Ordering::Relaxed);
    } else {
        // The status is in the 1XX-3XX range. Increment the `successes` field.
        state.sucesses.fetch_add(1, Ordering::Relaxed);
    }

    Ok(response)
}

/// A responder that will return the total number of `successes` and `errors`
/// in the `Counter` state.
async fn totals(request: Request<Counter>, _: Next<Counter>) -> via::Result {
    // Get a reference to the `Counter` state from the request. We don't need
    // to clone the state since we are consuming the request rather than
    // passing it as an argument to `next.call`.
    let state = request.state();

    // Load the current value of `errors` from the atomic integer.
    let errors = state.errors.load(Ordering::Relaxed);

    // Load the current value of `successes` from the atomic integer.
    let successes = state.sucesses.load(Ordering::Relaxed);

    // Create a new string to hold the message. Since we want the message
    // to be multiple lines, we'll use `writeln!` instead of `format!`.
    let mut message = String::new();

    writeln!(&mut message, "Errors: {}", errors)?;
    writeln!(&mut message, "Sucesses: {}", successes)?;

    // Return a string with the total number of `errors` and `successes`.
    Response::build().text(message)
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    let mut app = App::new(Counter {
        errors: Arc::new(AtomicU32::new(0)),
        sucesses: Arc::new(AtomicU32::new(0)),
    });

    // Add the `counter` middleware to the application. Since we are not
    // specifying an endpoint with the `at` method, this middleware will
    // be applied to all requests.
    app.include(counter);

    // Add the `totals` responder to the endpoint GET /totals.
    app.at("/totals").respond(via::get(totals));

    via::serve(app).listen(("127.0.0.1", 8080)).await
}
