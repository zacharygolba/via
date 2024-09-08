//! Aggregate the data of a body in-memory.

mod read_into_bytes;
mod read_into_string;

pub use read_into_bytes::ReadIntoBytes;
pub use read_into_string::ReadIntoString;
