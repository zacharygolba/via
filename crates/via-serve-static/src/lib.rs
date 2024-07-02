mod resolve;
mod respond;
mod stream_file;

use std::{path::PathBuf, sync::Arc};
use via::{Endpoint, Error, Result};

use crate::respond::{respond_to_get_request, respond_to_head_request};

/// The default file size threshold in bytes at which the server will eagerly
/// read the file into memory.
const DEFAULT_EAGER_READ_THRESHOLD: u64 = 1048576; // 1MB

/// The default timeout in seconds used when reading a file in chunks.
const DEFAULT_CHUNKED_READ_TIMEOUT: u64 = 60;

pub struct ServeStatic<'a, State> {
    chunked_read_timeout: u64,
    eager_read_threshold: u64,
    fall_through: bool,
    endpoint: Endpoint<'a, State>,
}

#[derive(Clone)]
pub(crate) struct ServerConfig {
    chunked_read_timeout: u64,
    eager_read_threshold: u64,
    fall_through: bool,
    path_param: &'static str,
    public_dir: PathBuf,
}

/// Returns a builder struct used to configure the static server middleware.
/// The provided `endpoint` must have a path parameter.
pub fn serve_static<State>(endpoint: Endpoint<State>) -> ServeStatic<State> {
    ServeStatic {
        chunked_read_timeout: DEFAULT_CHUNKED_READ_TIMEOUT,
        eager_read_threshold: DEFAULT_EAGER_READ_THRESHOLD,
        fall_through: true,
        endpoint,
    }
}

impl<'a, State> ServeStatic<'a, State>
where
    State: Send + Sync + 'static,
{
    /// Configures the timeout in seconds used when reading a file in chunks. The
    /// default value is 60 seconds.
    pub fn chunked_read_timeout(mut self, timeout: u64) -> Self {
        self.chunked_read_timeout = timeout;
        self
    }

    /// Configures the file size threshold in bytes at which the server will eagerly
    /// read the file into memory. The default value is 1MB.
    pub fn eager_read_threshold(mut self, threshold: u64) -> Self {
        self.eager_read_threshold = threshold;
        self
    }

    /// Configures whether or not to fall through to the next middleware if a file
    /// is not found or if the request is made with unsupported HTTP method. The
    /// default value is `true`.
    pub fn fall_through(mut self, fall_through: bool) -> Self {
        self.fall_through = fall_through;
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
        let mut public_dir: PathBuf = public_dir.try_into()?;
        let chunked_read_timeout = self.chunked_read_timeout;
        let eager_read_threshold = self.eager_read_threshold;
        let fall_through = self.fall_through;
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
            chunked_read_timeout,
            eager_read_threshold,
            fall_through,
            path_param,
            public_dir,
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
