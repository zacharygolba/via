use std::{io::ErrorKind as IoErrorKind, sync::Arc};
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
    let file_path = config.extract_path(&request)?;
    let file = match resolve_metadata(file_path).await {
        Ok(file) => file,
        Err(error) => {
            if config.fall_through && error.kind() == IoErrorKind::NotFound {
                return next.call(request).await;
            } else {
                return Err(Error::from_io_error(error));
            }
        }
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
    let file_path = config.extract_path(&request)?;
    let mut file = match resolve_file(file_path).await {
        Ok(file) => file,
        Err(error) => {
            if config.fall_through && error.kind() == IoErrorKind::NotFound {
                return next.call(request).await;
            } else {
                return Err(Error::from_io_error(error));
            }
        }
    };

    let content_length = file.metadata.len();
    let content_type = file.mime_type.to_string();

    if let Some(data) = file.data.take() {
        Response::build()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, content_length)
            .body(data)
            .end()
    } else {
        Response::build()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::TRANSFER_ENCODING, "chunked")
            .body(File::open(&file.path).await?)
            .end()
    }
}
