use http::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use httpdate::HttpDate;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;

use super::Response;
use crate::middleware;
use crate::Error;

type GenerateEtag = fn(&Metadata) -> Result<String, Error>;

/// A specialized response builder used to serve a single file from disk.
///
pub struct File {
    path: PathBuf,
    etag: Option<GenerateEtag>,
    content_type: Option<String>,
    with_last_modified: bool,
}

impl File {
    /// Specify the path at which the file we want to serve is located.
    ///
    pub fn open(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            etag: None,
            content_type: None,
            with_last_modified: false,
        }
    }

    /// Generate an etag by calling the provided function with a reference to
    /// the file's [Metadata].
    ///
    pub fn etag(self, f: GenerateEtag) -> Self {
        Self {
            etag: Some(f),
            ..self
        }
    }

    /// Set the value of the `Content-Type` header that will be included in the
    /// response.
    ///
    pub fn content_type(self, mime_type: String) -> Self {
        Self {
            content_type: Some(mime_type),
            ..self
        }
    }

    /// Include a `Last-Modified` header in the response.
    ///
    pub fn with_last_modified(self) -> Self {
        Self {
            with_last_modified: true,
            ..self
        }
    }

    /// Respond with the contents of this file.
    ///
    pub async fn serve(mut self) -> middleware::Result {
        let mut file = fs::File::open(&self.path).await.map_err(Error::from_io)?;
        let metadata = file.metadata().await.map_err(Error::from_io)?;

        // Allocate the exact capacity required to store the file in memory.
        // This is
        let mut data = Vec::with_capacity(metadata.len().try_into()?);

        file.read_to_end(&mut data).await.map_err(Error::from_io)?;

        let mut response = Response::build().header(CONTENT_LENGTH, data.len());

        if let Some(mime_type) = self.content_type.take() {
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
