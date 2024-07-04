use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate(inode: u64, size: u64, modified: SystemTime) -> Option<String> {
    let modified_timestamp = modified.duration_since(UNIX_EPOCH).ok()?;
    let modified_as_secs = modified_timestamp.as_secs();

    hash_etag(inode, size, modified_as_secs)
}

#[cfg(not(any(feature = "etag-md5", feature = "etag-sha1", feature = "etag-sha256")))]
fn hash_etag(inode: u64, size: u64, modified_as_secs: u64) -> Option<String> {
    Some(format!("{:x}-{:x}-{:x}", inode, size, modified_as_secs))
}

#[cfg(all(
    feature = "etag-md5",
    not(any(feature = "etag-sha1", feature = "etag-sha256"))
))]
fn hash_etag(inode: u64, size: u64, modified_as_secs: u64) -> Option<String> {
    use md5::{Digest, Md5};
    use std::io::Write;

    let mut hasher = Md5::new();
    let mut bytes = Vec::new();

    write!(&mut bytes, "{}-{}-{}", inode, size, modified_as_secs).ok()?;
    hasher.update(bytes);

    Some(format!("{:x}", hasher.finalize()))
}

#[cfg(all(
    feature = "etag-sha1",
    not(any(feature = "etag-md5", feature = "etag-sha256"))
))]
fn hash_etag(inode: u64, size: u64, modified_as_secs: u64) -> Option<String> {
    use sha1::{Digest, Sha1};
    use std::io::Write;

    let mut hasher = Sha1::new();
    let mut bytes = Vec::new();

    write!(&mut bytes, "{}-{}-{}", inode, size, modified_as_secs).ok()?;
    hasher.update(bytes);

    Some(format!("{:x}", hasher.finalize()))
}

#[cfg(all(
    feature = "etag-sha256",
    not(any(feature = "etag-md5", feature = "etag-sha1"))
))]
fn hash_etag(inode: u64, size: u64, modified_as_secs: u64) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::io::Write;

    let mut hasher = Sha256::new();
    let mut bytes = Vec::new();

    write!(&mut bytes, "{}-{}-{}", inode, size, modified_as_secs).ok()?;
    hasher.update(bytes);

    Some(format!("{:x}", hasher.finalize()))
}
