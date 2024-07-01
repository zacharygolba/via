use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};
use via::{http::header, Error, Next, Request, Response, Result};

use crate::{
    resolve::{resolve_file, resolve_metadata},
    stream_file::StreamFile,
    ServerConfig,
};

pub async fn respond_to_head_request<State>(
    config: Arc<ServerConfig>,
    request: Request<State>,
    next: Next<State>,
) -> Result<Response>
where
    State: Send + Sync + 'static,
{
    let path = build_path_from_request(&request, &config.public_dir, config.path_param)?;
    let file = match resolve_metadata(path).await {
        // The file does not exist and the server is configured to fall through
        // to the next middleware.
        Err(error) if config.fall_through && error.kind() == ErrorKind::NotFound => {
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

pub async fn respond_to_get_request<State>(
    config: Arc<ServerConfig>,
    request: Request<State>,
    next: Next<State>,
) -> Result<Response>
where
    State: Send + Sync + 'static,
{
    let path = build_path_from_request(&request, &config.public_dir, config.path_param)?;
    let file = match resolve_file(path, config.eager_read_threshold).await {
        // The file does not exist and the server is configured to fall through
        // to the next middleware.
        Err(error) if config.fall_through && error.kind() == ErrorKind::NotFound => {
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

    if let Some(data) = file.data {
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
        Response::stream(StreamFile::new(file.path))
            .header(header::CONTENT_TYPE, content_type)
            .end()
    }
}

fn build_path_from_request<State>(
    request: &Request<State>,
    public_dir: &Path,
    path_param_name: &str,
) -> Result<PathBuf> {
    let path_param = request.param(path_param_name).required()?;
    Ok(public_dir.join(path_param.trim_end_matches('/')))
}
