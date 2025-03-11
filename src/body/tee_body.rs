use bytes::{Buf, Bytes};
use http_body::{Body, Frame};
use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

use super::BoxBody;
use crate::error::DynError;

/// A boxed body that copies bytes from each data frame into a dyn
/// [`AsyncWrite`](AsyncWrite).
///
// This struct needs refactored to contain a state enum to ensure that
// we're able to (flush if necessary) and shutdown the sink when body
// stops producing frames...
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
    pub fn new(body: BoxBody, sink: impl AsyncWrite + Send + Sync + 'static) -> Self {
        Self {
            io: Box::pin(sink),
            body: BoxBody::new(body),
            state: IoState::Writeable(VecDeque::with_capacity(2)),
        }
    }

    pub fn cap(self) -> BoxBody {
        self.body
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
        let mut done = false;
        let mut body_err = None;

        loop {
            let backlog = loop {
                return match state {
                    IoState::Writeable(bufs) => break bufs,
                    IoState::Shutdown => match this.io.as_mut().poll_shutdown(context) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(Ok(())) => {
                            *state = IoState::Closed;
                            Poll::Ready(body_err)
                        }
                        Poll::Ready(Err(error)) => {
                            *state = IoState::Closed;
                            Poll::Ready(body_err.or_else(|| Some(Err(error.into()))))
                        }
                    },
                    IoState::Closed => Poll::Ready(None),
                };
            };

            if let Some(mut next) = backlog.pop_front() {
                match this.io.as_mut().poll_write(context, &next) {
                    Poll::Pending => {
                        backlog.push_front(next);
                        // Placeholder for tracing...
                    }
                    Poll::Ready(Ok(len)) => {
                        if len < next.len() {
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
            } else if done {
                *state = IoState::Shutdown;
                continue;
            }

            match Pin::new(&mut this.body).poll_frame(context) {
                Poll::Pending => break Poll::Pending,
                Poll::Ready(None) => {
                    done = true;
                }
                Poll::Ready(Some(Ok(frame))) => {
                    if let Some(next) = frame.data_ref() {
                        backlog.push_back(next.clone());
                    }

                    break Poll::Ready(Some(Ok(frame)));
                }
                Poll::Ready(Some(err @ Err(_))) => {
                    *state = IoState::Shutdown;
                    body_err = Some(err);
                }
            };
        }
    }
}

impl Debug for TeeBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TeeBody").finish()
    }
}
