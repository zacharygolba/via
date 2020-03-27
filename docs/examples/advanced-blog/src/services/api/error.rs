use diesel::result::{DatabaseErrorKind, Error as DieselError};
use tokio_diesel::AsyncError;
use via::system::*;

use super::Document;

pub struct DatabaseErrorCode<'a> {
    kind: &'a DatabaseErrorKind,
}

pub async fn handler(context: Context, next: Next) -> Result {
    next.call(context).await.or_else(catch)
}

fn as_diesel_error(error: &Error) -> Option<&DieselError> {
    if let AsyncError::Error(value) = error.source().downcast_ref()? {
        Some(value)
    } else {
        None
    }
}

fn catch(error: Error) -> Result {
    match as_diesel_error(&error) {
        Some(DieselError::DatabaseError(kind, _)) => {
            let status = u16::from(DatabaseErrorCode { kind });
            Err(error.status(status).json())
        }
        Some(DieselError::NotFound) => Document::new(()).status(404).respond(),
        Some(_) | None => Err(error.json()),
    }
}

impl<'a> From<DatabaseErrorCode<'a>> for u16 {
    fn from(error: DatabaseErrorCode<'a>) -> u16 {
        use DatabaseErrorKind::*;

        match error.kind {
            ForeignKeyViolation | UniqueViolation => 400,
            _ => 500,
        }
    }
}
