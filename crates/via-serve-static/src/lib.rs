mod etag;
mod respond;
mod static_file;
mod stream_file;

use bitflags::bitflags;
use std::{path::PathBuf, sync::Arc};
use via::{Endpoint, Error, Result};

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
    path_param: &'static str,
    public_dir: PathBuf,
    flags: Flags,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct Flags: u8 {
        const FALL_THROUGH = 0b00000001;
        const INCLUDE_ETAG = 0b00000010;
    }
}

/// Returns a builder struct used to configure the static server middleware.
/// The provided `endpoint` must have a path parameter.
pub fn serve_static<State>(endpoint: Endpoint<State>) -> ServeStatic<State> {
    ServeStatic {
        eager_read_threshold: 1048576, // 1MB
        read_stream_timeout: 60,       // 60 seconds
        flags: Flags::FALL_THROUGH | Flags::INCLUDE_ETAG,
        endpoint,
    }
}

impl<'a, State> ServeStatic<'a, State>
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

    /// Configures whether or not to include an ETag header in the response. The
    /// default value is `true`.
    pub fn include_etag(mut self, value: bool) -> Self {
        self.flags.set(Flags::INCLUDE_ETAG, value);
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
    pub fn serve<T>(mut self, public_dir: T) -> Result<()>
    where
        Error: From<T::Error>,
        T: TryInto<PathBuf>,
    {
        let ServeStatic {
            eager_read_threshold,
            read_stream_timeout,
            flags,
            ..
        } = self;
        let mut public_dir: PathBuf = public_dir.try_into()?;
        let path_param = match self.endpoint.param() {
            Some(param) => param,
            None => {
                return Err(Error::new(
                    "The provided endpoint does not have a path parameter.".to_owned(),
                ))
            }
        };

        if public_dir.is_relative() {
            let current_dir = std::env::current_dir()?;
            public_dir = current_dir.join(public_dir).canonicalize()?;
        }

        let config = ServerConfig {
            eager_read_threshold,
            read_stream_timeout,
            path_param,
            public_dir,
            flags,
        };

        self.endpoint.respond({
            let config = Arc::new(config.clone());
            via::head(move |request, next| {
                let config = Arc::clone(&config);
                respond_to_head_request(config, request, next)
            })
        });

        self.endpoint.respond({
            let config = Arc::new(config.clone());
            via::get(move |request, next| {
                let config = Arc::clone(&config);
                respond_to_get_request(config, request, next)
            })
        });

        Ok(())
    }
}
