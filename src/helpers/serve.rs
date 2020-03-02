use crate::{Context, Handler, Next};
use std::path::{Path, PathBuf};

pub fn dir(path: impl AsRef<Path>) -> impl Handler {
    let path = path.as_ref().to_path_buf();

    |context: Context, next: Next| next.call(context)
}

pub fn file(path: impl AsRef<Path>) -> impl Handler {
    let path = path.as_ref().to_path_buf();

    |context: Context, next: Next| next.call(context)
}
