use http::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use httpdate::HttpDate;
use std::path::PathBuf;
use via::body::{BufferBody, HttpBody};
use via::{Error, Next, Pipe, Request, Response};

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
                return Err(Error::not_found(Box::new(error)));
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
) -> Result<Response, Error>
where
    State: Send + Sync + 'static,
{
    let mut file = try_unwrap_file!(config, request, next, {
        let path = build_path_from_request(&request, &config)?;
        StaticFile::metadata(path, config.flags).await
    });

    let mut headers = vec![
        (CONTENT_LENGTH, Some(file.size.to_string())),
        (CONTENT_TYPE, Some(file.mime_type.clone())),
        (ETAG, file.etag.take()),
    ];

    if config.flags.contains(Flags::INCLUDE_LAST_MODIFIED) {
        headers.push((
            LAST_MODIFIED,
            file.modified_at.map(|at| HttpDate::from(at).to_string()),
        ))
    }

    Response::build().headers(headers).finish()
}

pub async fn respond_to_get_request<T>(
    config: ServerConfig,
    request: Request<T>,
    next: Next<T>,
) -> Result<Response, Error> {
    let mut file = try_unwrap_file!(config, request, next, {
        let path = build_path_from_request(&request, &config)?;
        let flags = config.flags;
        let eager_read_threshold = config.eager_read_threshold;

        StaticFile::open(path, flags, eager_read_threshold).await
    });

    let mut headers = vec![
        (CONTENT_TYPE, Some(file.mime_type.clone())),
        (ETAG, file.etag.take()),
    ];

    if config.flags.contains(Flags::INCLUDE_LAST_MODIFIED) {
        headers.push((
            LAST_MODIFIED,
            file.modified_at.map(|at| HttpDate::from(at).to_string()),
        ))
    }

    if let Some(data) = file.data.take() {
        // The file was small enough to be eagerly read into memory. We can
        // respond immediately with the entire vector of bytes as the
        // response body.
        return Response::build()
            .header(CONTENT_LENGTH, file.size)
            .headers(headers)
            .body(HttpBody::Inline(BufferBody::from(data)));
    }

    // The file is too large to be eagerly read into memory. Stream the
    // file data from disk into the response body.
    let stream = StreamFile::new(file.path, config.read_stream_timeout);

    stream.pipe(Response::build().headers(headers))
}

fn build_path_from_request<T>(
    request: &Request<T>,
    config: &ServerConfig,
) -> Result<PathBuf, Error> {
    let path_param = request.param(&config.path_param).into_result()?;
    Ok(config.public_dir.join(path_param.trim_end_matches('/')))
}
