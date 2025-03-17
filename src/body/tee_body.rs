use bytes::{Buf, Bytes};
use http_body::{Body, Frame};
use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, ErrorKind};
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
    Shutdown(Option<DynError>),
    Writeable(VecDeque<Bytes>),
}

fn broken_pipe() -> DynError {
    Box::new(io::Error::from(ErrorKind::BrokenPipe))
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
        let state = &mut this.state;

        loop {
            let backlog = 'writable: {
                let next = match state {
                    IoState::Writeable(deque) => break 'writable deque,
                    IoState::Shutdown(last) => last.take().map(Err),
                    IoState::Closed => return Poll::Ready(None),
                };

                return match this.io.as_mut().poll_shutdown(context) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Ok(())) => {
                        *state = IoState::Closed;
                        Poll::Ready(next)
                    }
                    Poll::Ready(Err(error)) => {
                        *state = IoState::Closed;
                        Poll::Ready(next.or_else(|| Some(Err(error.into()))))
                    }
                };
            };

            let next = match Pin::new(&mut this.body).poll_frame(context) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    if backlog.is_empty() {
                        *state = IoState::Shutdown(None);
                        continue;
                    }

                    None
                }
                Poll::Ready(Some(Ok(frame))) => {
                    if let Some(data) = frame.data_ref() {
                        backlog.push_back(data.clone());
                    }

                    Some(Ok(frame))
                }
                Poll::Ready(Some(Err(error))) => {
                    backlog.clear();
                    *state = IoState::Shutdown(Some(error));
                    continue;
                }
            };

            if let Some(mut buf) = backlog.pop_front() {
                let bytes_written = match this.io.as_mut().poll_write(context, &buf) {
                    Poll::Pending => {
                        return next.map_or(Poll::Pending, |result| Poll::Ready(Some(result)));
                    }
                    Poll::Ready(Ok(n)) => n,
                    Poll::Ready(Err(error)) => {
                        backlog.clear();
                        *state = IoState::Shutdown(Some(error.into()));
                        continue;
                    }
                };
                let remaining = buf.remaining();

                if bytes_written == 0 && remaining > 0 {
                    *state = IoState::Shutdown(Some(broken_pipe()));
                    continue;
                }

                if bytes_written < remaining {
                    buf.advance(bytes_written);
                    backlog.push_front(buf);
                }
            }

            return Poll::Ready(next);
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
