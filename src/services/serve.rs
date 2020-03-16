use crate::{self as via, error::Error, prelude::*, Response};
use hyper::body::Body;
use std::{io::ErrorKind, path::PathBuf, str::FromStr};
use tokio::{fs::File, io::AsyncRead};

pub struct Static {
    root: Object,
}

struct Object {
    path: PathBuf,
}

impl FromStr for Object {
    type Err = Error;

    #[inline]
    fn from_str(value: &str) -> Result<Object, Error> {
        let mut path = PathBuf::new();

        for segment in value.split('/') {
            if segment.starts_with("..") {
                bail!("unsafe path segment starting with '..' found in {}", value);
            } else if segment.contains('\\') {
                bail!("unsafe path segment containing '\\' found in {}", value);
            } else {
                path.push(segment);
            }
        }

        if path.is_dir() {
            path.push("index.html");
        }

        if path.is_file() && path.extension().is_none() {
            path.set_extension("html");
        }

        Ok(Object { path })
    }
}

#[service]
impl Static {
    pub fn new(path: &'static str) -> Static {
        match path.parse() {
            Ok(root) => Static { root },
            Err(e) => panic!("{}", e),
        }
    }

    #[http(GET, "/*object")]
    async fn serve(&self, object: Object) -> Result<impl Respond> {
        let (mut sender, body) = Body::channel();
        let file = File::open(&object.path).await.or_else(|e| match e.kind() {
            ErrorKind::NotFound => Err(Error::message("Not Found").status(404)),
            ErrorKind::PermissionDenied => Err(Error::message("Forbidden").status(403)),
            _ => Err(e.into()),
        })?;

        tokio::spawn(async move {});

        Ok(Response::new(body))
    }
}
