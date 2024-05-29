use std::path::{Component, Path, PathBuf};
use tokio::fs::{self, File};
use via::prelude::*;

pub struct ServeStatic {
    root: PathBuf,
}

#[service]
impl ServeStatic {
    pub fn new<T>(path: T) -> Result<ServeStatic>
    where
        Error: From<T::Error>,
        T: TryInto<PathBuf>,
    {
        let mut root = path.try_into()?;

        if root.is_relative() {
            root = normalize_path(&std::env::current_dir()?.join(root));
        }

        Ok(ServeStatic { root })
    }

    #[endpoint(GET, "/*path")]
    async fn serve(&self, path: String, context: Context, next: Next) -> Result {
        let absolute_path = self.root.join(path.trim_start_matches('/'));
        let file_path = resolve_file_path(&absolute_path).await?;
        let mut response = match try_open_file(&file_path).await? {
            Some(file) => file.respond()?,
            None => return next.call(context).await,
        };

        response.headers_mut().insert(
            "Content-Type",
            mime_guess::from_path(&file_path)
                .first_or_octet_stream()
                .to_string()
                .parse()?,
        );

        Ok(response)
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
