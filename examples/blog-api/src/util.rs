use diesel::result::{DatabaseErrorKind, Error as DieselError};
use http::StatusCode;
use via::error::Sanitizer;

pub fn error_sanitizer(sanitizer: &mut Sanitizer) {
    // Print the original message to stderr. In production you probably want
    // to use env_logger, tracing, or something similar.
    eprintln!("error: {}", sanitizer);

    // Configure the sanitizer to generate a JSON response.
    sanitizer.use_json();

    // If the error occurred during a database operation, set the appropriate
    // status code or obsfuscate the message depending on the nature of the
    // error.
    let Some(database_error) = sanitizer.source().and_then(|error| error.downcast_ref()) else {
        return;
    };

    match database_error {
        DieselError::DatabaseError(kind, _) => match kind {
            DatabaseErrorKind::CheckViolation | DatabaseErrorKind::NotNullViolation => {
                sanitizer.set_status_code(StatusCode::BAD_REQUEST);
            }
            DatabaseErrorKind::ForeignKeyViolation => {
                sanitizer.set_status_code(StatusCode::UNPROCESSABLE_ENTITY);
            }
            DatabaseErrorKind::UniqueViolation => {
                sanitizer.set_status_code(StatusCode::CONFLICT);
            }
            _ => {
                // Some other database error occurred. To be safe, use the
                // canonical reason phrase of the status code associated with
                // the error as the error message.
                sanitizer.use_canonical_reason();
            }
        },

        // The requested resource does not exist.
        DieselError::NotFound => {
            sanitizer.set_status_code(StatusCode::NOT_FOUND);
        }

        // The error occured for some other reason.
        _ => {}
    }
}
