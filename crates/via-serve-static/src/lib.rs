mod resolve;
mod respond;

use std::{path::PathBuf, sync::Arc};
use via::{Endpoint, Error, Request, Result};

use crate::respond::{respond_to_get_request, respond_to_head_request};

pub struct ServeStatic<'a> {
    fall_through: bool,
    endpoint: Endpoint<'a>,
}

#[derive(Clone)]
pub(crate) struct ServerConfig {
    fall_through: bool,
    path_param: &'static str,
    public_dir: PathBuf,
}

/// Returns a builder struct used to configure the static server middleware.
/// The provided `endpoint` must have a path parameter.
pub fn serve_static(endpoint: Endpoint) -> ServeStatic {
    ServeStatic {
        fall_through: true,
        endpoint,
    }
}

impl<'a> ServeStatic<'a> {
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

impl ServerConfig {
    /// Extracts the relative file path from the request using the configured
    /// path parameter name and joins it with the public directory.
    pub(crate) fn extract_path(&self, request: &Request) -> Result<PathBuf> {
        let relative_path = request.param(&self.path_param).required()?;
        Ok(self.public_dir.join(relative_path.trim_matches('/')))
    }
}
