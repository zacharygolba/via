mod etag;
mod respond;
mod static_file;
mod stream_file;

use bitflags::bitflags;
use std::path::Path;
use std::sync::Arc;
use via::{BoxError, Endpoint};

use crate::respond::{respond_to_get_request, respond_to_head_request};

pub struct ServeStatic<'a, State> {
    eager_read_threshold: u64,
    read_stream_timeout: u64,
    endpoint: Endpoint<'a, State>,
    flags: Flags,
}

#[derive(Clone)]
pub(crate) struct ServerConfig {
    eager_read_threshold: u64,
    read_stream_timeout: u64,
    path_param: Box<str>,
    public_dir: Arc<Path>,
    flags: Flags,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct Flags: u8 {
        const FALL_THROUGH          = 0b00000001;
        const INCLUDE_ETAG          = 0b00000010;
        const INCLUDE_LAST_MODIFIED = 0b00000100;
    }
}

/// Returns a builder struct used to configure the static server middleware.
/// The provided `endpoint` must have a path parameter.
pub fn serve_static<State>(endpoint: Endpoint<State>) -> ServeStatic<State> {
    ServeStatic {
        eager_read_threshold: 1048576, // 1MB
        read_stream_timeout: 60,       // 60 seconds
        flags: Flags::FALL_THROUGH,
        endpoint,
    }
}

impl<State> ServeStatic<'_, State>
where
    State: Send + Sync + 'static,
{
    /// Configures the file size threshold in bytes at which the server will eagerly
    /// read the file into memory. The default value is 1MB.
    pub fn eager_read_threshold(mut self, threshold: u64) -> Self {
        self.eager_read_threshold = threshold;
        self
    }

    /// Configures whether or not to fall through to the next middleware if a file
    /// is not found or if the request is made with unsupported HTTP method. The
    /// default value is `true`.
    pub fn fall_through(mut self, value: bool) -> Self {
        self.flags.set(Flags::FALL_THROUGH, value);
        self
    }

    /// Configures the server to include a Last-Modified header in the response.
    pub fn include_last_modified(mut self) -> Self {
        self.flags.insert(Flags::INCLUDE_LAST_MODIFIED);
        self
    }

    /// Configures the server to include an ETag header in the response.
    pub fn include_etag(mut self) -> Self {
        self.flags.insert(Flags::INCLUDE_ETAG);
        self
    }

    /// Configures the timeout in seconds used when streaming a file to the
    /// client. The default value is 60 seconds.
    pub fn read_stream_timeout(mut self, timeout: u64) -> Self {
        self.read_stream_timeout = timeout;
        self
    }

    /// Attempts to add the static server middleware at the provided `endpoint`. If
    /// the provided `public_dir` is a relative path, it will be resolved relative to
    /// the current working directory. If the `public_dir` is not a directory or the
    /// `location` does not have a path parameter, an error will be returned.
    pub fn serve<P>(mut self, public_dir: P) -> Result<(), BoxError>
    where
        P: AsRef<Path>,
    {
        let ServeStatic {
            eager_read_threshold,
            read_stream_timeout,
            flags,
            ..
        } = self;
        let mut public_dir = public_dir.as_ref().to_path_buf();
        let path_param = self.endpoint.param().map_or_else(
            || {
                let message = "The provided endpoint does not have a path parameter.";
                Err(BoxError::from(message.to_owned()))
            },
            |value| Ok(value.to_owned().into_boxed_str()),
        )?;

        if public_dir.is_relative() {
            let current_dir = std::env::current_dir()?;
            public_dir = current_dir.join(public_dir).canonicalize()?;
        }

        let config = ServerConfig {
            public_dir: public_dir.into(),
            eager_read_threshold,
            read_stream_timeout,
            path_param,
            flags,
        };

        self.endpoint.respond({
            let config = config.clone();
            via::head(move |request, next| {
                let config = config.clone();
                respond_to_head_request(config, request, next)
            })
        });

        self.endpoint.respond({
            let config = config.clone();
            via::get(move |request, next| {
                let config = config.clone();
                respond_to_get_request(config, request, next)
            })
        });

        Ok(())
    }
}
