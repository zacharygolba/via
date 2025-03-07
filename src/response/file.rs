use futures::TryStreamExt;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use httpdate::HttpDate;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;

use super::Response;
use crate::body::{Pipe, MAX_FRAME_LEN};
use crate::error::Error;
use crate::middleware;

/// A function pointer used to generate an etag.
///
type GenerateEtag = fn(&Metadata) -> Result<Option<String>, Error>;

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

    /// Respond with a stream of the file contents in chunks.
    ///
    pub async fn stream(mut self) -> middleware::Result {
        let file = fs::File::open(&self.path).await.map_err(Error::from_io)?;
        let meta = file.metadata().await.map_err(Error::from_io)?;

        self.stream_file(&meta, file)
    }

    /// Respond with the entire contents of the file loaded in to memory.
    ///
    pub async fn send(mut self) -> middleware::Result {
        let mut file = fs::File::open(&self.path).await.map_err(Error::from_io)?;
        let meta = file.metadata().await.map_err(Error::from_io)?;
        let len = isize::try_from(meta.len())? as usize;

        let mut data = Vec::with_capacity(len);

        file.read_to_end(&mut data).await.map_err(Error::from_io)?;
        self.respond_from_memory(&meta, data)
    }

    /// Respond with the contents of the file.
    ///
    /// If the file is larger than the provided `max_alloc_size` in bytes, it
    /// will be streamed over the socket with chunked transfer encoding.
    ///
    pub async fn serve(mut self, max_alloc_size: usize) -> middleware::Result {
        let mut file = fs::File::open(&self.path).await.map_err(Error::from_io)?;
        let meta = file.metadata().await.map_err(Error::from_io)?;
        let len = isize::try_from(meta.len())? as usize;

        if len > max_alloc_size {
            self.stream_file(&meta, file)
        } else {
            let mut data = Vec::with_capacity(len);

            file.read_to_end(&mut data).await.map_err(Error::from_io)?;
            self.respond_from_memory(&meta, data)
        }
    }

    fn gen_etag(&self, meta: &Metadata) -> Result<Option<String>, Error> {
        match self.etag.as_ref() {
            Some(f) => f(meta),
            None => Ok(None),
        }
    }

    fn stream_file(&mut self, meta: &Metadata, mut file: fs::File) -> middleware::Result {
        let mut response = Response::build();

        if let Some(mime_type) = self.content_type.take() {
            response = response.header(CONTENT_TYPE, mime_type);
        }

        if let Some(etag) = self.gen_etag(meta)? {
            response = response.header(ETAG, etag);
        }

        if self.with_last_modified {
            let last_modified = HttpDate::from(meta.modified()?);
            response = response.header(LAST_MODIFIED, last_modified.to_string());
        }

        file.set_max_buf_size(MAX_FRAME_LEN * 2);

        ReaderStream::new(file)
            .map_err(|error| error.into())
            .pipe(response)
    }

    fn respond_from_memory(&mut self, meta: &Metadata, data: Vec<u8>) -> middleware::Result {
        let mut response = Response::build().header(CONTENT_LENGTH, data.len());

        if let Some(mime_type) = self.content_type.take() {
            response = response.header(CONTENT_TYPE, mime_type);
        }

        if let Some(etag) = self.gen_etag(meta)? {
            response = response.header(ETAG, etag);
        }

        if self.with_last_modified {
            let last_modified = HttpDate::from(meta.modified()?);
            response = response.header(LAST_MODIFIED, last_modified.to_string());
        }

        response.body(data)
    }
}
