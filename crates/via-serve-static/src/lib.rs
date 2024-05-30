use std::path::{Component, Path, PathBuf};
use tokio::fs::{self, File};
use via::{prelude::*, routing::Location};

pub struct ServeStatic<'a> {
    location: Location<'a>,
}

struct StaticServer {
    path_param: &'static str,
    public_dir: PathBuf,
}

impl<'a> ServeStatic<'a> {
    pub fn new(location: Location<'a>) -> Self {
        ServeStatic { location }
    }

    pub fn serve<T>(mut self, public_dir: T) -> Result<()>
    where
        Error: From<T::Error>,
        T: TryInto<PathBuf>,
    {
        let mut public_dir = public_dir.try_into()?;
        let path_param = match self.location.param() {
            Some(param) => param,
            None => via::bail!("location is missing path parameter"),
        };

        if public_dir.is_relative() {
            let current_dir = std::env::current_dir()?;
            public_dir = normalize_path(&current_dir.join(public_dir));
        }

        self.location.include(StaticServer {
            path_param,
            public_dir,
        });

        Ok(())
    }
}

impl Middleware for StaticServer {
    fn call(&self, context: Context, next: Next) -> via::BoxFuture<Result> {
        let path_param = self.path_param;
        let public_dir = self.public_dir.clone();

        Box::pin(async move {
            let path_param_value = context.params().get::<String>(path_param)?;
            let absolute_path = public_dir.join(path_param_value.trim_start_matches('/'));
            let file_path = resolve_file_path(&absolute_path).await?;
            let file = match try_open_file(&file_path).await? {
                Some(file) => file,
                None => return next.call(context).await,
            };

            file.respond()?.with_header(
                "Content-Type",
                mime_guess::from_path(&file_path)
                    .first_or_octet_stream()
                    .to_string(),
            )
        })
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

async fn resolve_file_path(path: &Path) -> Result<PathBuf> {
    let mut file_path = path.to_path_buf();

    if file_path.is_dir() {
        file_path = file_path.join("index.html");
        if !fs::try_exists(&file_path).await? {
            file_path = file_path.with_extension("htm");
        }
    }

    Ok(file_path)
}

async fn try_open_file(path: &Path) -> Result<Option<File>> {
    use std::io::ErrorKind;

    match File::open(&path).await {
        Ok(file) => Ok(Some(file)),
        Err(error) => match error.kind() {
            ErrorKind::PermissionDenied => Err(Error::from(error).status(403)),
            ErrorKind::NotFound => Ok(None),
            _ => Err(error.into()),
        },
    }
}
