use http::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use httpdate::HttpDate;
use std::fs::Metadata;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;

use super::Response;
use crate::{middleware, Error};

type GenerateEtag = fn(&Metadata) -> Result<String, Error>;

/// Serve a single file from disk.
///
pub struct File {
    path: PathBuf,
    etag: Option<GenerateEtag>,
    content_type: Option<String>,
    with_last_modified: bool,
}

fn wrap_io_error(error: io::Error) -> Error {
    match error.kind() {
        ErrorKind::PermissionDenied => Error::forbidden(error.into()),
        ErrorKind::FileTooLarge => Error::payload_too_large(error.into()),
        ErrorKind::NotFound => Error::not_found(error.into()),
        _ => Error::internal_server_error(error.into()),
    }
}

impl File {
    pub fn open(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            etag: None,
            content_type: None,
            with_last_modified: false,
        }
    }

    pub fn etag(self, f: GenerateEtag) -> Self {
        Self {
            etag: Some(f),
            ..self
        }
    }

    pub fn content_type(self, mime_type: &str) -> Self {
        Self {
            content_type: Some(mime_type.to_owned()),
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
        let mut file = fs::File::open(&self.path).await.map_err(wrap_io_error)?;
        let metadata = file.metadata().await?;

        let mut data = match metadata.len().try_into() {
            Ok(capacity) => Vec::with_capacity(capacity),
            Err(error) => return Err(Error::payload_too_large(error.into())),
        };

        file.read_to_end(&mut data).await?;

        let mut response = Response::build().header(CONTENT_LENGTH, data.len());

        if let Some(mime_type) = self.content_type.as_ref() {
            response = response.header(CONTENT_TYPE, mime_type);
        }

        if let Some(f) = self.etag.as_ref() {
            response = response.header(ETAG, f(&metadata)?);
        }

        if self.with_last_modified {
            let last_modified = HttpDate::from(metadata.modified()?);
            response = response.header(LAST_MODIFIED, last_modified.to_string());
        }

        response.body(data)
    }
}
