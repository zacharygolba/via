use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::ReadIntoBytes;
use crate::Result;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadIntoString {
    future: ReadIntoBytes,
}

impl ReadIntoString {
    pub(crate) fn new(future: ReadIntoBytes) -> Self {
        Self { future }
    }
}

impl Future for ReadIntoString {
    type Output = Result<String>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        // Get a mutable reference to `Self`.
        let this = self.get_mut();
        // Get a mutable reference to the `future` field.
        let future = &mut this.future;

        // Pin `future` on the stack and poll it.
        Pin::new(future).poll(context).map(|result| {
            // Convert the returned bytes to a string if it is valid UTF-8.
            Ok(String::from_utf8(result?)?)
        })
    }
}
