use std::{
    fs::{self, File},
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::PathBuf,
    time::SystemTime,
};
use tokio::task;

use crate::{etag, Flags};

const UTF_8_PREFFERED_MIME_TYPES: [&str; 6] = [
    "application/javascript",
    "text/html",
    "text/css",
    "text/plain",
    "text/csv",
    "text/tab-separated-values",
];

pub(crate) struct StaticFile {
    pub data: Option<Vec<u8>>,
    pub path: PathBuf,
    pub size: u64,
    pub etag: Option<String>,
    pub mime_type: String,
    #[cfg_attr(not(feature = "last-modified"), allow(dead_code))]
    pub modified_at: Option<SystemTime>,
}

fn get_mime_type_from_path(path: &PathBuf) -> String {
    let mut mime_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();

    if UTF_8_PREFFERED_MIME_TYPES.contains(&&*mime_type) {
        mime_type += "; charset=utf-8";
    }

    mime_type
}

fn resolve_file_blocking(
    path: PathBuf,
    flags: Flags,
    eager_read_threshold: u64,
) -> io::Result<StaticFile> {
    let mut file = resolve_metadata_blocking(path, flags)?;

    if file.size < eager_read_threshold {
        let mut buf = Vec::with_capacity(file.size as usize);

        File::open(&file.path)?.read_to_end(&mut buf)?;
        file.data = Some(buf);
    }

    Ok(file)
}

fn resolve_metadata_blocking(path: PathBuf, flags: Flags) -> io::Result<StaticFile> {
    let path = resolve_path_blocking(path);
    let metadata = fs::metadata(&path)?;

    let data = None;
    let size = metadata.len();
    let inode = metadata.ino();
    let mime_type = get_mime_type_from_path(&path);
    let modified_at = metadata.modified().ok();
    let etag = if flags.contains(Flags::INCLUDE_ETAG) {
        modified_at.and_then(|time| etag::generate(inode, size, time))
    } else {
        None
    };

    Ok(StaticFile {
        data,
        path,
        size,
        etag,
        mime_type,
        modified_at,
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

impl StaticFile {
    /// Resolves a file path to a `ResolvedFile` without loading the file data
    /// into memory.
    pub async fn metadata(path: PathBuf, flags: Flags) -> io::Result<StaticFile> {
        let future = task::spawn_blocking(move || {
            // Run the blocking operation in a separate thread.
            resolve_metadata_blocking(path, flags)
        });

        future.await?
    }

    /// Resolves a file path to a `StaticFile` and conditionally loads the file
    /// a data into memory if the file size is less than `eager_read_threshold`.
    pub async fn open(
        path: PathBuf,
        flags: Flags,
        eager_read_threshold: u64,
    ) -> io::Result<StaticFile> {
        let future = task::spawn_blocking(move || {
            // Run the blocking operation in a separate thread.
            resolve_file_blocking(path, flags, eager_read_threshold)
        });

        future.await?
    }
}
