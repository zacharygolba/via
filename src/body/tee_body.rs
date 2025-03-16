use bytes::{Buf, Bytes};
use http_body::{Body, Frame};
use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

use super::BoxBody;
use crate::error::DynError;

/// A boxed body that writes each data frame into a dyn
/// [`AsyncWrite`](AsyncWrite).
///
pub struct TeeBody {
    io: Pin<Box<dyn AsyncWrite + Send + Sync>>,
    body: BoxBody,
    state: IoState,
}

enum IoState {
    Closed,
    Shutdown,
    Writeable(VecDeque<Bytes>),
}

impl TeeBody {
    pub fn new(body: BoxBody, io: impl AsyncWrite + Send + Sync + 'static) -> Self {
        Self {
            io: Box::pin(io),
            body,
            state: IoState::Writeable(VecDeque::with_capacity(2)),
        }
    }
}

impl Body for TeeBody {
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();

        let io = &mut this.io;
        let state = &mut this.state;
        let mut is_done = false;
        let mut next_frame = None;
        let mut polled_frame = false;

        loop {
            let backlog = match state {
                IoState::Writeable(bufs) => bufs,
                IoState::Shutdown => {
                    return match io.as_mut().poll_shutdown(context) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(Ok(())) => {
                            *state = IoState::Closed;
                            Poll::Ready(next_frame)
                        }
                        Poll::Ready(Err(e)) => {
                            *state = IoState::Closed;
                            Poll::Ready(next_frame.or_else(|| Some(Err(e.into()))))
                        }
                    };
                }
                IoState::Closed => {
                    return Poll::Ready(None);
                }
            };

            if let Some(mut next) = backlog.pop_front() {
                match io.as_mut().poll_write(context, &next) {
                    Poll::Pending => {
                        backlog.push_front(next);
                        // Placeholder for tracing...
                        if polled_frame {
                            return Poll::Pending;
                        }
                    }
                    Poll::Ready(Ok(len)) => {
                        if len < next.remaining() {
                            next.advance(len);
                            backlog.push_front(next);
                        }
                    }
                    Poll::Ready(Err(error)) => {
                        let _ = &error; // Placeholder for tracing...
                        *state = IoState::Shutdown;
                        continue;
                    }
                }
            } else if is_done {
                *state = IoState::Shutdown;
                continue;
            }

            if next_frame.is_some() {
                return Poll::Ready(next_frame);
            }

            match Pin::new(&mut this.body).poll_frame(context) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    is_done = true;
                    polled_frame = true;
                }
                Poll::Ready(Some(Ok(frame))) => {
                    if let Some(next) = frame.data_ref() {
                        backlog.push_back(next.clone());
                    }

                    next_frame = Some(Ok(frame));
                    polled_frame = true;
                }
                Poll::Ready(Some(Err(error))) => {
                    *state = IoState::Shutdown;
                    next_frame = Some(Err(error));
                    polled_frame = true;
                }
            };
        }
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.body.size_hint()
    }
}

impl Debug for TeeBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TeeBody").finish()
    }
}
