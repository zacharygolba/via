use diesel::result::{DatabaseErrorKind, Error as DieselError};
use http::StatusCode;
use via::error::Sanitize;

/// Sanitizes details about database errors that
pub fn with_error_sanitizer(error: Sanitize) -> Sanitize {
    // Print the original message to stderr. In production you probably want
    // to use env_logger, tracing, or something similar.
    eprintln!("error: {}", error);

    // Respond with json and sanitize potentially sensitive error messages.
    error.as_json().map(|sanitize, source| {
        match source.downcast_ref() {
            Some(DieselError::DatabaseError(kind, _)) => {
                if let Some(status_code) = status_for_database_error_kind(kind) {
                    // The requested operation violates a database constraint.
                    sanitize.with_status_code(status_code)
                } else {
                    // Opaque internal server error.
                    sanitize.with_canonical_reason()
                }
            }

            // The requested resource does not exist.
            Some(DieselError::NotFound) => sanitize.with_status_code(StatusCode::NOT_FOUND),

            // The error occured for some other reason.
            _ => sanitize.with_canonical_reason(),
        }
    })
}

fn status_for_database_error_kind(kind: &DatabaseErrorKind) -> Option<StatusCode> {
    match kind {
        DatabaseErrorKind::CheckViolation | DatabaseErrorKind::NotNullViolation => {
            Some(StatusCode::BAD_REQUEST)
        }
        DatabaseErrorKind::ForeignKeyViolation => Some(StatusCode::UNPROCESSABLE_ENTITY),
        DatabaseErrorKind::UniqueViolation => Some(StatusCode::CONFLICT),
        _ => None,
    }
}
