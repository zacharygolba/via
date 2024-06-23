use std::{io::ErrorKind, sync::Arc};
use tokio::fs::File;
use via::{http::header, Error, Next, Request, Response, Result};

use crate::{
    resolve::{resolve_file, resolve_metadata},
    ServerConfig,
};

pub async fn respond_to_head_request(
    config: Arc<ServerConfig>,
    request: Request,
    next: Next,
) -> Result<Response> {
    let ServerConfig { fall_through, .. } = *config;
    let file_path = config.extract_path(&request)?;
    let file = match resolve_metadata(file_path).await {
        // The file does not exist and the server is configured to fall through
        // to the next middleware.
        Err(error) if fall_through && error.kind() == ErrorKind::NotFound => {
            return next.call(request).await;
        }
        // An error occurred while attempting to resolve the file metadata. Return
        // an error response with the appropriate status code.
        Err(error) => {
            return Err(Error::from_io_error(error));
        }
        // The file metadata was successfully resolved.
        Ok(file) => file,
    };

    Response::build()
        .header(header::CONTENT_TYPE, file.mime_type.to_string())
        .header(header::CONTENT_LENGTH, file.metadata.len())
        .end()
}

pub async fn respond_to_get_request(
    config: Arc<ServerConfig>,
    request: Request,
    next: Next,
) -> Result<Response> {
    let ServerConfig { fall_through, .. } = *config;
    let file_path = config.extract_path(&request)?;
    let mut file = match resolve_file(file_path).await {
        // The file does not exist and the server is configured to fall through
        // to the next middleware.
        Err(error) if fall_through && error.kind() == ErrorKind::NotFound => {
            return next.call(request).await;
        }
        // An error occurred while attempting to resolve the file. Return an
        // error response with the appropriate status code.
        Err(error) => {
            return Err(Error::from_io_error(error));
        }
        // The file was successfully resolved.
        Ok(file) => file,
    };

    let content_length = file.metadata.len();
    let content_type = file.mime_type.to_string();

    if let Some(data) = file.data.take() {
        // The file was small enough to be eagerly read into memory. We can respond
        // immediately with the entire vector of bytes as the response body.
        Response::build()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, content_length)
            .body(data)
            .end()
    } else {
        // The file is too large to be eagerly read into memory. We will stream the
        // file data from disk to the response body.
        Response::build()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::TRANSFER_ENCODING, "chunked")
            .body(File::open(&file.path).await?)
            .end()
    }
}
