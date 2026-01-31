use diesel::result::{DatabaseErrorKind, Error as DieselError};
use http::StatusCode;
use std::fmt::{self, Display, Formatter};
use via::error::Sanitizer;

#[derive(Debug)]
pub struct InvalidIdError;

pub fn forbidden<T>() -> via::Result<T> {
    via::raise!(403, message = "access denied.");
}

pub fn unauthorized<T>() -> via::Result<T> {
    via::raise!(401, message = "authentication is required.");
}

pub fn error_sanitizer(sanitizer: &mut Sanitizer) {
    // Print the original message to stderr. In production you probably want
    // to use env_logger, tracing, or something similar.
    eprintln!("error: {}", sanitizer);

    // Configure the sanitizer to generate a JSON response.
    sanitizer.use_json();

    let Some(error) = sanitizer.source() else {
        return;
    };

    if let Some(diesel_error) = error.downcast_ref() {
        match diesel_error {
            DieselError::DatabaseError(kind, _) => match kind {
                DatabaseErrorKind::CheckViolation | DatabaseErrorKind::NotNullViolation => {
                    sanitizer.set_status(StatusCode::BAD_REQUEST);
                }
                DatabaseErrorKind::ForeignKeyViolation => {
                    sanitizer.set_status(StatusCode::UNPROCESSABLE_ENTITY);
                }
                DatabaseErrorKind::UniqueViolation => {
                    sanitizer.set_status(StatusCode::CONFLICT);
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
                sanitizer.set_status(StatusCode::NOT_FOUND);
            }

            // The error occured for some other reason.
            _ => {}
        }
    } else if error.is::<chrono::ParseError>() {
        sanitizer.set_status(StatusCode::BAD_REQUEST);
        sanitizer.set_message("Invalid timestamp.");
    }
}

impl std::error::Error for InvalidIdError {}

impl Display for InvalidIdError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "invalid uuid")
    }
}
