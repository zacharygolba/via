use mime_guess::Mime;
use std::{
    fs::Metadata,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};
use tokio::fs::{self, File};
use via::{
    http::{header, Method, StatusCode},
    middleware::{BoxFuture, Middleware},
    Endpoint, Error, Next, Request, Response, Result,
};

pub struct ServeStatic<'a> {
    fall_through: bool,
    endpoint: Endpoint<'a>,
}

struct ServerConfig {
    fall_through: bool,
    path_param: &'static str,
    public_dir: PathBuf,
}

struct StaticServer {
    config: Arc<ServerConfig>,
}

async fn try_open_file(path: &Path) -> Result<Option<File>> {
    use std::io::ErrorKind;

    match File::open(&path).await {
        Ok(file) => Ok(Some(file)),
        Err(error) => match error.kind() {
            ErrorKind::PermissionDenied => {
                let mut error = Error::from(error);

                *error.status_mut() = StatusCode::FORBIDDEN;
                Err(error)
            }
            ErrorKind::NotFound => Ok(None),
            _ => Err(error.into()),
        },
    }
}

async fn try_read_metadata(path: &Path) -> Result<Option<Metadata>> {
    use std::io::ErrorKind;

    match fs::metadata(path).await {
        Ok(file) => Ok(Some(file)),
        Err(error) => match error.kind() {
            ErrorKind::PermissionDenied => {
                let mut error = Error::from(error);

                *error.status_mut() = StatusCode::FORBIDDEN;
                Err(error)
            }
            ErrorKind::NotFound => Ok(None),
            _ => Err(error.into()),
        },
    }
}

impl<'a> ServeStatic<'a> {
    /// Returns a builder struct used to configure the static server middleware.
    /// The provided `endpoint` must have a path parameter.
    pub fn new(endpoint: Endpoint<'a>) -> Self {
        ServeStatic {
            fall_through: true,
            endpoint,
        }
    }

    /// Configures whether or not to fall through to the next middleware if a file
    /// is not found or if the request is made with unsupported HTTP method. The
    /// default value is `true`.
    pub fn fall_through(mut self, fall_through: bool) -> Self {
        self.fall_through = fall_through;
        self
    }

    /// Attempts to add the static server middleware at the provided `location`. If
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

        self.endpoint.include(StaticServer {
            config: Arc::new(ServerConfig {
                fall_through,
                path_param,
                public_dir,
            }),
        });

        Ok(())
    }
}

impl StaticServer {
    /// Returns an absolute path based on the relative path extracted from the
    /// path parameter.
    fn expand_path(&self, path: &str) -> PathBuf {
        self.config.public_dir.join(path.trim_start_matches('/'))
    }

    /// Locates the file based on the path parameter extracted from the request.
    /// If the path parameter is a directory, it will attempt to locate an index
    /// file.
    async fn locate_file(&self, request: &Request) -> Result<(Mime, PathBuf)> {
        let path_param_value = request.param(&self.config.path_param).required()?;
        let mut file_path = self.expand_path(&path_param_value);

        if file_path.is_dir() {
            file_path = file_path.join("index.html");
            // Eagerly determine whether or not index.html exists in order to
            // support the alternative index.htm extension.
            if !fs::try_exists(&file_path).await? {
                file_path = file_path.with_extension("htm");
            }
        }

        Ok((
            mime_guess::from_path(&file_path).first_or_octet_stream(),
            file_path,
        ))
    }

    async fn respond_to_get_request(&self, request: Request, next: Next) -> Result<Response> {
        let (mime_type, file_path) = self.locate_file(&request).await?;

        if let Some(file) = try_open_file(&file_path).await? {
            return Response::build()
                .header(header::CONTENT_TYPE, mime_type.to_string())
                .header(header::TRANSFER_ENCODING, "chunked")
                .body(file)
                .end();
        }

        if self.config.fall_through {
            next.call(request).await
        } else {
            Response::text("Not Found").status(404).end()
        }
    }

    async fn respond_to_head_request(&self, request: Request, next: Next) -> Result<Response> {
        let (mime_type, file_path) = self.locate_file(&request).await?;

        if let Some(metadata) = try_read_metadata(&file_path).await? {
            return Response::build()
                .header(header::CONTENT_TYPE, mime_type.to_string())
                .header(header::CONTENT_LENGTH, metadata.len())
                .end();
        }

        if self.config.fall_through {
            next.call(request).await
        } else {
            Response::text("Not Found").status(404).end()
        }
    }
}

impl Middleware for StaticServer {
    fn call(self: Pin<&Self>, request: Request, next: Next) -> BoxFuture<Result<Response>> {
        let middleware = StaticServer {
            config: Arc::clone(&self.config),
        };

        Box::pin(async move {
            if request.method() == Method::GET {
                middleware.respond_to_get_request(request, next).await
            } else if request.method() == Method::HEAD {
                middleware.respond_to_head_request(request, next).await
            } else if middleware.config.fall_through {
                next.call(request).await
            } else {
                Response::text("Method Not Allowed").status(405).end()
            }
        })
    }
}
