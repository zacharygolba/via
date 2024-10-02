use diesel::result::Error as DieselError;
use via::error::Error;
use via::http::StatusCode;

use crate::State;

/// Used with the InspectErrorBoundary to log errors that occur on /api routes.
///
pub fn inspect_error(error: &Error, _: &State) {
    // In production you'll likely want to use tracing or report to some error
    // tracking service.
    eprintln!("Error: {}", error);
}

/// Used with the MapErrorBoundary to map errors that occur on /api routes. This
/// function ensures that errors that occur in the /api namespace respond with
/// JSON and do not leak sensitive information to the client.
///
pub fn map_error(error: Error, _: &State) -> Error {
    // Define the error argument as a mutable variable.
    let mut error = error;

    match error.source().downcast_ref() {
        // The error occurred because a record was not found in the
        // database, set the status to 404 Not Found.
        Some(DieselError::NotFound) => {
            error = Error::new("Not Found".to_string());
            error.set_status(StatusCode::NOT_FOUND);
        }

        // The error occurred because of a database error. Return a
        // new error with an opaque message.
        Some(_) => {
            error = Error::new("Internal Server Error".to_string());
        }

        // The error occurred for some other reason.
        None => {}
    }

    // Configure the error to respond with JSON.
    error.respond_with_json();

    // Return the modified error.
    error
}
