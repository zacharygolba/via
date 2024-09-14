use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::ReadIntoBytes;
use crate::Error;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadIntoString {
    future: ReadIntoBytes,
}

impl ReadIntoString {
    pub(crate) fn new(future: ReadIntoBytes) -> Self {
        Self { future }
    }
}

impl ReadIntoString {
    fn project(self: Pin<&mut Self>) -> Pin<&mut ReadIntoBytes> {
        // Get a mutable reference to `Self`.
        let this = self.get_mut();
        // Get a mutable reference to the `future` field.
        let ptr = &mut this.future;

        Pin::new(ptr)
    }
}

impl Future for ReadIntoString {
    type Output = Result<String, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        self.project().poll(context).map(|result| {
            // Convert the returned bytes to a string if it is valid UTF-8.
            Ok(String::from_utf8(result?)?)
        })
    }
}
