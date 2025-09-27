use diesel::result::Error as DieselError;
use http::StatusCode;
use via::error::Sanitize;

/// Sanitizes details about database errors that
pub fn with_error_sanitizer(error: Sanitize) -> Sanitize {
    // Print the original message to stderr. In production you probably want
    // to use env_logger, tracing, or something similar.
    eprintln!("error: {}", error);

    // Respond with json and sanitize potentially sensitive error messages.
    error.as_json().map(|respond, source| {
        match source.downcast_ref() {
            // The requested operation violates a database constraint.
            Some(DieselError::DatabaseError(_, info)) if info.constraint_name().is_some() => {
                respond.with_status_code(StatusCode::BAD_REQUEST)
            }

            // The requested resource does not exist.
            Some(DieselError::NotFound) => respond.with_status_code(StatusCode::NOT_FOUND),

            // The error occured for some other reason.
            _ => respond.with_canonical_reason(),
        }
    })
}
