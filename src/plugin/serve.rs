use crate::{self as via, error::Error, prelude::*};
use futures::stream::{self, Stream, StreamExt};
use hyper::Body;
use std::{
    io::ErrorKind,
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::{fs::File, io::AsyncRead};

pub struct Serve {
    root: Resource,
}

struct Resource {
    path: PathBuf,
}

macro_rules! open {
    ($resource:expr, $context:expr, $next:expr) => {
        match File::open($resource).await {
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                return "Forbidden".status(403).respond();
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return $next.call($context).await;
            }
            result => result?,
        }
    };
}

impl Deref for Resource {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl FromStr for Resource {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        todo!()
    }
}

#[service]
impl Serve {
    #[http(GET, "/*resource")]
    async fn get(&self, resource: Resource, context: Context, next: Next) -> Result<impl Respond> {
        let path = self.root.join(&*resource);
        let file = open!(&path, context, next);

        "".respond()
    }

    #[http(HEAD, "/*resource")]
    async fn head(&self, resource: Resource, context: Context, next: Next) -> Result<impl Respond> {
        let file = open!(self.root.join(&*resource), context, next);

        "".respond()
    }
}
