use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::Response;
use crate::middleware;
use crate::Error;

fn respond_with_io_error(error: io::Error) -> Error {
    let kind = error.kind();
    let source = Box::new(error);

    match kind {
        ErrorKind::PermissionDenied => Error::forbidden(source),
        ErrorKind::NotFound => Error::not_found(source),
        _ => Error::internal_server_error(source),
    }
}

/// Serve a single file from disk.
///
pub struct File {
    path: PathBuf,
    mime_type: Option<String>,
}

impl File {
    pub fn open(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            mime_type: None,
        }
    }

    pub fn mime_type(self, mime_type: &str) -> Self {
        Self {
            mime_type: Some(mime_type.to_owned()),
            ..self
        }
    }

    pub async fn serve(self) -> middleware::Result {
        let data = fs::read(&self.path).await.map_err(respond_with_io_error)?;
        let mut response = Response::build().header(CONTENT_LENGTH, data.len());

        if let Some(mime_type) = &self.mime_type {
            response = response.header(CONTENT_TYPE, mime_type);
        }

        response.body(data.into())
    }
}
