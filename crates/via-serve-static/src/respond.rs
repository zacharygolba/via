use httpdate::HttpDate;
use std::path::PathBuf;
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
    config: ServerConfig,
    request: Request<State>,
    next: Next<State>,
) -> Result<Response>
where
    State: Send + Sync + 'static,
{
    let mut file = try_unwrap_file!(config, request, next, {
        let path = build_path_from_request(&request, &config)?;
        StaticFile::metadata(path, config.flags).await
    });
    let optional_headers = get_optional_headers(&mut file, &config.flags);

    Response::builder()
        .header(header::CONTENT_TYPE, file.mime_type)
        .header(header::CONTENT_LENGTH, file.size)
        .headers(optional_headers)
        .finish()
}

pub async fn respond_to_get_request<State>(
    config: ServerConfig,
    request: Request<State>,
    next: Next<State>,
) -> Result<Response>
where
    State: Send + Sync + 'static,
{
    let mut file = try_unwrap_file!(config, request, next, {
        let path = build_path_from_request(&request, &config)?;
        let flags = config.flags;
        let eager_read_threshold = config.eager_read_threshold;

        StaticFile::open(path, flags, eager_read_threshold).await
    });
    let optional_headers = get_optional_headers(&mut file, &config.flags);

    match file.data.take() {
        Some(data) => {
            // The file was small enough to be eagerly read into memory. We can
            // respond immediately with the entire vector of bytes as the
            // response body.
            Response::builder()
                .header(header::CONTENT_TYPE, file.mime_type)
                .header(header::CONTENT_LENGTH, file.size)
                .headers(optional_headers)
                .body(data)
                .finish()
        }
        None => {
            // The file is too large to be eagerly read into memory. Stream the
            // file data from disk into the response body.
            let stream = StreamFile::new(file.path, config.read_stream_timeout);

            Response::stream(stream)
                .header(header::CONTENT_TYPE, file.mime_type)
                .headers(optional_headers)
                .finish()
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

fn get_optional_headers(
    file: &mut StaticFile,
    flags: &Flags,
) -> impl Iterator<Item = (HeaderName, String)> {
    let last_modified = if flags.contains(Flags::INCLUDE_LAST_MODIFIED) {
        file.modified_at.map(|time| {
            let http_date = HttpDate::from(time);
            (header::LAST_MODIFIED, http_date.to_string())
        })
    } else {
        None
    };

    // We don't need to check if the `INCLUDE_ETAG` flag is set because the
    // value of `file.etag` is `None` when the flag is not set.
    let etag = file.etag.take().map(|etag| (header::ETAG, etag));

    last_modified.into_iter().chain(etag)
}
