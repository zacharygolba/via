use super::{Error, Source};
use http::{Response, StatusCode};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Json,
}

#[derive(Clone, Copy, Debug)]
pub struct Respond {
    pub(crate) format: Option<Format>,
    pub(crate) status: u16,
}

#[derive(Eq, PartialEq, Hash)]
struct SerializedError {
    message: String,
}

fn respond<T>(error: Error) -> Result<Response<T>, Error>
where
    Vec<u8>: Into<T>,
{
    let Error { respond, .. } = &error;
    let mut response = Response::new(match respond.format {
        Some(Format::Json) => serde_json::to_vec(&error)?.into(),
        None => error.to_string().into_bytes().into(),
    });

    *response.status_mut() = StatusCode::from_u16(respond.status)?;
    Ok(response)
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let errors: HashSet<_> = self.chain().map(SerializedError::from).collect();
        let mut state = serializer.serialize_struct("Errors", 1)?;

        state.serialize_field("errors", &errors)?;
        state.end()
    }
}

impl Default for Respond {
    fn default() -> Self {
        Respond {
            format: None,
            status: 500,
        }
    }
}

impl<T> From<Error> for Response<T>
where
    Vec<u8>: Into<T>,
{
    fn from(error: Error) -> Response<T> {
        respond(error).unwrap_or_else(|error| {
            let bytes = b"Internal Server Error".to_vec();
            let mut response = Response::new(bytes.into());

            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            eprintln!("{}", error);
            response
        })
    }
}

impl<'a> From<&'a Source> for SerializedError {
    fn from(error: &'a Source) -> Self {
        SerializedError {
            message: error.to_string(),
        }
    }
}

impl Serialize for SerializedError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 1)?;

        state.serialize_field("message", &self.message)?;
        state.end()
    }
}
