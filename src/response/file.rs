use http::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use httpdate::HttpDate;
use std::fs::Metadata;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::Response;
use crate::{middleware, Error};

/// Serve a single file from disk.
///
pub struct File {
    path: PathBuf,
    etag: Option<fn(&Metadata) -> Result<String, Error>>,
    mime_type: Option<String>,
    with_last_modified: bool,
}

fn wrap_io_error(error: io::Error) -> Error {
    let kind = error.kind();
    let source = Box::new(error);

    match kind {
        ErrorKind::PermissionDenied => Error::forbidden(source),
        ErrorKind::NotFound => Error::not_found(source),
        _ => Error::internal_server_error(source),
    }
}

impl File {
    pub fn open(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            etag: None,
            mime_type: None,
            with_last_modified: false,
        }
    }

    pub fn etag(self, f: fn(&Metadata) -> Result<String, Error>) -> Self {
        Self {
            etag: Some(f),
            ..self
        }
    }

    pub fn mime_type(self, mime_type: &str) -> Self {
        Self {
            mime_type: Some(mime_type.to_owned()),
            ..self
        }
    }

    pub fn with_last_modified(self) -> Self {
        Self {
            with_last_modified: true,
            ..self
        }
    }

    pub async fn serve(self) -> middleware::Result {
        let path = self.path.as_path();
        let data = fs::read(path).await.map_err(wrap_io_error)?;
        let meta = if self.etag.is_some() || self.with_last_modified {
            Some(fs::metadata(path).await.map_err(wrap_io_error)?)
        } else {
            None
        };

        let mut response = Response::build().header(CONTENT_LENGTH, data.len());

        if let Some(mime_type) = self.mime_type.as_ref() {
            response = response.header(CONTENT_TYPE, mime_type);
        }

        if let Some(metadata) = meta.as_ref() {
            if self.with_last_modified {
                let last_modified = HttpDate::from(metadata.modified()?);
                response = response.header(LAST_MODIFIED, last_modified.to_string());
            }

            if let Some(f) = self.etag.as_ref() {
                response = response.header(ETAG, f(metadata)?);
            }
        }

        response.body(data.into())
    }
}
