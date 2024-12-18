use diesel::result::{DatabaseErrorKind, Error as DieselError};
use via::http::StatusCode;
use via::Error;

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
    match error.source().downcast_ref() {
        // The error occurred because a database record was not found.
        Some(DieselError::NotFound) => error
            .as_json()
            .with_status(StatusCode::NOT_FOUND)
            .use_canonical_reason(),

        // The occurred because of some kind of constraint violation.
        Some(DieselError::DatabaseError(
            DatabaseErrorKind::ForeignKeyViolation
            | DatabaseErrorKind::UniqueViolation
            | DatabaseErrorKind::NotNullViolation,
            _,
        )) => error
            .as_json()
            .with_status(StatusCode::BAD_REQUEST)
            .use_canonical_reason(),

        // The error occurred because of some other kind of database error.with a
        Some(_) => error.as_json().use_canonical_reason(),

        // The error occurred for some other reason.
        None => error.as_json(),
    }
}
