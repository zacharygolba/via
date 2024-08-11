use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

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

    fn project(self: Pin<&mut Self>) -> Pin<&mut ReadIntoBytes> {
        // Get a mutable reference to self.
        let this = self.get_mut();
        let future = &mut this.future;

        // Project the buffer and stream.
        Pin::new(future)
    }
}

impl Future for ReadIntoString {
    type Output = Result<String>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        self.project().poll(context).map(|result| {
            result.and_then(|bytes| {
                let utf8 = Vec::from(bytes);
                Ok(String::from_utf8(utf8)?)
            })
        })
    }
}
