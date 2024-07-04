use std::{path::PathBuf, sync::Arc};
use via::{
    http::{header, HeaderName},
    Next, Request, Response, Result,
};

use crate::{static_file::StaticFile, stream_file::StreamFile, Flags, ServerConfig};

/// Unwraps the result of a file operation and handles early return logic based
/// on the server configuration.
macro_rules! try_unwrap_file {
    (
        // Should be `crate::ServerConfig`.
        $config:ident,
        // Should be `via::Request`.
        $request:ident,
        // Should be `via::Next`.
        $next:ident,
        // Should evaluate to `crate::StaticFile`.
        $f:expr
    ) => {{
        use std::io::ErrorKind;
        use via::Error;

        use crate::Flags;

        match $f {
            Err(error)
                if $config.flags.contains(Flags::FALL_THROUGH)
                    && error.kind() == ErrorKind::NotFound =>
            {
                // The file does not exist and the server is configured to fall
                // through to the next middleware.
                return $next.call($request).await;
            }
            Err(error) => {
                // An error occurred while attempting to resolve the file metadata.
                // Return an error response with the appropriate status code.
                return Err(Error::from_io_error(error));
            }
            Ok(file) => {
                // The file metadata was successfully resolved.
                file
            }
        }
    }};
}

pub async fn respond_to_head_request<State>(
    config: Arc<ServerConfig>,
    request: Request<State>,
    next: Next<State>,
) -> Result<Response>
where
    State: Send + Sync + 'static,
{
    let file = try_unwrap_file!(config, request, next, {
        let path = build_path_from_request(&request, &config)?;
        StaticFile::metadata(path, config.flags).await
    });
    let optional_headers = get_optional_headers(&file, &config.flags);

    Response::build()
        .header(header::CONTENT_TYPE, file.mime_type)
        .header(header::CONTENT_LENGTH, file.size)
        .headers(optional_headers)
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
    let file = try_unwrap_file!(config, request, next, {
        let path = build_path_from_request(&request, &config)?;
        let flags = config.flags;
        let eager_read_threshold = config.eager_read_threshold;

        StaticFile::open(path, flags, eager_read_threshold).await
    });
    let optional_headers = get_optional_headers(&file, &config.flags);

    match file.data {
        Some(data) => {
            // The file was small enough to be eagerly read into memory. We can
            // respond immediately with the entire vector of bytes as the
            // response body.
            Response::build()
                .header(header::CONTENT_TYPE, file.mime_type)
                .header(header::CONTENT_LENGTH, file.size)
                .headers(optional_headers)
                .body(data)
                .end()
        }
        None => {
            // The file is too large to be eagerly read into memory. Stream the
            // file data from disk into the response body.
            let stream = StreamFile::new(file.path, config.read_stream_timeout);

            Response::stream(stream)
                .header(header::CONTENT_TYPE, file.mime_type)
                .headers(optional_headers)
                .end()
        }
    }
}

fn build_path_from_request<State>(
    request: &Request<State>,
    config: &ServerConfig,
) -> Result<PathBuf> {
    let path_param = request.param(config.path_param).required()?;
    Ok(config.public_dir.join(path_param.trim_end_matches('/')))
}

#[cfg(not(feature = "last-modified"))]
fn get_last_modified_header(_: &StaticFile, _: &Flags) -> Option<(HeaderName, String)> {
    None
}

#[cfg(feature = "last-modified")]
fn get_last_modified_header(file: &StaticFile, flags: &Flags) -> Option<(HeaderName, String)> {
    Some((
        header::LAST_MODIFIED,
        httpdate::fmt_http_date(file.modified_at?),
    ))
}

fn get_optional_headers(
    file: &StaticFile,
    flags: &Flags,
) -> impl Iterator<Item = (HeaderName, String)> {
    file.etag
        .clone()
        .map(|etag| (header::ETAG, etag))
        .into_iter()
        .chain(get_last_modified_header(file, flags))
}
