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
    state: IoState,
    io: Pin<Box<dyn AsyncWrite + Send + Sync>>,
    body: BoxBody,
    backlog: VecDeque<Bytes>,
}

enum IoState {
    Closed,
    Shutdown,
    Writeable,
}

fn broken_pipe() -> DynError {
    Box::new(io::Error::from(ErrorKind::BrokenPipe))
}

impl TeeBody {
    pub fn new(body: BoxBody, io: impl AsyncWrite + Send + Sync + 'static) -> Self {
        Self {
            io: Box::pin(io),
            body,
            state: IoState::Writeable,
            backlog: VecDeque::with_capacity(2),
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
        let backlog = &mut this.backlog;
        let mut next = None;

        loop {
            match state {
                IoState::Writeable => {}
                IoState::Shutdown => {
                    backlog.clear();
                    return match io.as_mut().poll_shutdown(context) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(Ok(())) => {
                            *state = IoState::Closed;
                            Poll::Ready(next)
                        }
                        Poll::Ready(Err(e)) => {
                            *state = IoState::Closed;
                            Poll::Ready(next.or_else(|| Some(Err(e.into()))))
                        }
                    };
                }
                IoState::Closed => {
                    return Poll::Ready(None);
                }
            };

            if let Some(front) = backlog.front_mut() {
                match io.as_mut().poll_write(context, front) {
                    Poll::Pending => {
                        // Placeholder for tracing...
                        //
                        // Something along the lines of:
                        // tracing::info!("TeeBody: io is not yet ready for writes.");
                        //
                        return Poll::Pending;
                    }
                    Poll::Ready(Ok(len)) => {
                        let remaining = front.remaining();

                        if len == 0 && remaining > len {
                            *state = IoState::Shutdown;
                            next = Some(Err(broken_pipe()));
                            continue;
                        }

                        if len < remaining {
                            front.advance(len);
                        } else {
                            backlog.pop_front();
                        }
                    }
                    Poll::Ready(Err(error)) => {
                        let _ = &error; // Placeholder for tracing...
                        *state = IoState::Shutdown;
                        continue;
                    }
                }
            }

            if next.is_some() {
                return Poll::Ready(next);
            }

            match Pin::new(&mut this.body).poll_frame(context) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    *state = IoState::Shutdown;

                    if !backlog.is_empty() {
                        context.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Some(Ok(frame))) => {
                    if let Some(data) = frame.data_ref() {
                        if backlog.len() == backlog.capacity() {
                            // Placeholder for tracing...
                            if cfg!(debug_assertions) {
                                println!("TeeBody: allocating for backlog");
                            }
                        }

                        backlog.push_back(data.clone());
                    }

                    next = Some(Ok(frame));
                }
                Poll::Ready(Some(Err(error))) => {
                    *state = IoState::Shutdown;
                    next = Some(Err(error));
                }
            }
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
