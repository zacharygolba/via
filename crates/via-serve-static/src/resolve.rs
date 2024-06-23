use mime_guess::Mime;
use std::{
    fs::{self, File, Metadata},
    io::{self, Read},
    path::PathBuf,
};
use tokio::task;

/// The maximum file size that will be eagerly read into memory. As a performance
/// optimization, small files will be read into memory immediately to avoid the
/// overhead of calling `tokio::task::spawn_blocking` multiple times.
///
/// The current value is set to `10MB`.
const EAGER_READ_THRESHOLD: u64 = 1024 * 1024 * 10;

pub(crate) struct ResolvedFile {
    pub data: Option<Vec<u8>>,
    pub path: PathBuf,
    pub metadata: Metadata,
    pub mime_type: Mime,
}

/// Resolves a file path to a `ResolvedFile` and conditionally loads the file a
/// data into memory if the file size is less than `EAGER_READ_THRESHOLD` constant.
pub async fn resolve_file(path: PathBuf) -> io::Result<ResolvedFile> {
    task::spawn_blocking(|| resolve_file_blocking(path)).await?
}

/// Resolves a file path to a `ResolvedFile` without loading the file data into memory.
pub async fn resolve_metadata(path: PathBuf) -> io::Result<ResolvedFile> {
    task::spawn_blocking(|| resolve_metadata_blocking(path)).await?
}

fn resolve_file_blocking(path: PathBuf) -> io::Result<ResolvedFile> {
    let mut resolved_file = resolve_metadata_blocking(path)?;
    let content_length = resolved_file.metadata.len();

    if content_length < EAGER_READ_THRESHOLD {
        let mut buffer = Vec::new();
        let mut file = File::open(&resolved_file.path)?;

        file.read_to_end(&mut buffer)?;
        resolved_file.data = Some(buffer);
    }

    Ok(resolved_file)
}

fn resolve_metadata_blocking(path: PathBuf) -> io::Result<ResolvedFile> {
    let path = resolve_path_blocking(path);
    let metadata = fs::metadata(&path)?;
    let mime_type = mime_guess::from_path(&path).first_or_octet_stream();

    Ok(ResolvedFile {
        path,
        metadata,
        mime_type,
        data: None,
    })
}

/// Resolves a path to a file on the file system. If the path is a directory, it will
/// attempt to resolve an `index.html` or `index.htm` file. If the path is missing an
/// extension, it will attempt to resolve it as an HTML file.
fn resolve_path_blocking(path: PathBuf) -> PathBuf {
    let mut path = path;

    if path.is_dir() {
        // The path is a directory. Check and see if there's an index.html file.
        path = path.join("index.html");

        if !path.exists() {
            // There wasn't an index.html at the root of the directory.
            // We'll fallback to index.htm if it exists.
            path = path.join("index.htm");
        }
    } else if !path.exists() && path.extension().is_none() {
        // The file doesn't exist and there isn't an extension in `path`. Try to
        // resolve it as an HTML file.
        path = path.with_extension("html");

        if !path.exists() {
            // The file doesn't exist with an `.html` extension. Try `.htm`.
            path = path.with_extension("htm");
        }
    }

    path
}
