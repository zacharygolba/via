use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::BoxError;

/// Converts an `impl Stream` to an `impl Body`.
///
#[must_use = "streams do nothing unless polled"]
pub struct StreamBody<T> {
    stream: T,
}

fn size_hint_as_u64((lower, upper): (usize, Option<usize>)) -> (Option<u64>, Option<u64>) {
    (
        lower.try_into().ok(),
        upper.and_then(|value| value.try_into().ok()),
    )
}

impl<T> StreamBody<T> {
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T> StreamBody<T> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe {
            let this = self.get_unchecked_mut();
            Pin::new_unchecked(&mut this.stream)
        }
    }
}

impl<T> Body for StreamBody<T>
where
    T: Stream<Item = Result<Frame<Bytes>, BoxError>> + Send + Sync,
{
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_next(context)
    }

    fn size_hint(&self) -> SizeHint {
        match size_hint_as_u64(self.stream.size_hint()) {
            (None, _) => SizeHint::new(),
            (Some(low), None) => {
                let mut hint = SizeHint::new();

                hint.set_lower(low);
                hint
            }
            (Some(low), Some(high)) => {
                let mut hint = SizeHint::new();

                hint.set_lower(low);
                hint.set_upper(high);
                hint
            }
        }
    }
}

impl<T> Debug for StreamBody<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamBody").finish()
    }
}
