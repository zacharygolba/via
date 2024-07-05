use std::time::{SystemTime, UNIX_EPOCH};

/// Defines the `hash_etag` function using the specified `hasher` from `module`.
#[cfg_attr(
    not(any(feature = "etag-md5", feature = "etag-sha1", feature = "etag-sha256")),
    allow(unused_macros)
)]
macro_rules! define_hash_fn {
    (
        // The module to import the hasher from.
        $module:path,
        // The name of the hasher to use. Should impl `Digest`.
        $hasher:ident
    ) => {
        fn hash_etag(etag: u64) -> Option<String> {
            use $module::{$hasher, Digest};

            let mut hasher = $hasher::new();

            // Update the hasher with the etag as a string.
            hasher.update(etag.to_string());
            let hashed_etag = hasher.finalize();

            // Return the hashed etag as a hexadecimal string.
            Some(format!("{:x}", hashed_etag))
        }
    };
}

/// Generate an etag using the specified inode, size, and modified time.
pub fn generate(inode: u64, size: u64, modified: SystemTime) -> Option<String> {
    // Convert the modified time to seconds since the UNIX epoch.
    let modified = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();

    // Combine the inode, modified, and size to generate the etag. This approach
    // is similar to the one used by Apache and NGINX.
    hash_etag((inode ^ (inode >> 32)) ^ (modified ^ (modified >> 32)) ^ size)
}

#[cfg(feature = "etag-md5")]
define_hash_fn!(md5, Md5);

#[cfg(feature = "etag-sha1")]
define_hash_fn!(sha1, Sha1);

#[cfg(feature = "etag-sha256")]
define_hash_fn!(sha2, Sha256);

/// Return the etag as a hexadecimal string if no etag hashing features are enabled.
#[cfg(not(any(feature = "etag-md5", feature = "etag-sha1", feature = "etag-sha256")))]
fn hash_etag(etag: u64) -> Option<String> {
    Some(format!("{:x}", etag))
}
