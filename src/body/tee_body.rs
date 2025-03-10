use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
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
    body: BoxBody,
    sink: Pin<Box<dyn AsyncWrite + Send + Sync>>,
    next: Option<Bytes>,
}

impl TeeBody {
    pub fn new(body: BoxBody, sink: impl AsyncWrite + Send + Sync + 'static) -> Self {
        Self {
            body: BoxBody::new(body),
            sink: Box::pin(sink),
            next: None,
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
        let mut next = loop {
            if let Some(data) = this.next.take() {
                if data.is_empty() {
                    this.next = None;
                } else {
                    break data;
                }
            } else {
                // The only reason this is safe is because we know that
                // RequestBody and ResponseBody are both Unpin.
                //
                // It's possible to have an unsafe pin projection here if you
                // supply an !Unpin stream to `Pipe::pipe` and then supply a
                // sink to Response::tee. So long as hyper::body::Incoming is
                // Unpin,
                let ready = match Pin::new(&mut this.body).poll_frame(context) {
                    Poll::Ready(Some(Ok(frame))) => frame,
                    poll @ (Poll::Pending | Poll::Ready(None) | Poll::Ready(Some(Err(_)))) => {
                        if this.sink.as_mut().poll_shutdown(context)? == Poll::Pending {
                            println!("pending shutdown");
                            return Poll::Pending;
                        } else {
                            println!("sink shutdown");
                            return poll;
                        }
                    }
                };

                this.next = match ready.into_data() {
                    Err(trailers) => return Poll::Ready(Some(Ok(trailers))),
                    Ok(data) => Some(data),
                };
            }
        };

        match this.sink.as_mut().poll_write(context, &next)? {
            Poll::Pending => {
                this.next = Some(next);
                Poll::Pending
            }
            Poll::Ready(len) => {
                if len == next.len() {
                    Poll::Ready(Some(Ok(Frame::data(next))))
                } else {
                    let data = next.split_to(len);
                    this.next = Some(next);
                    Poll::Ready(Some(Ok(Frame::data(data))))
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl Debug for TeeBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TeeBody").finish()
    }
}
