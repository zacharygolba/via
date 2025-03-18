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
    Writable(VecDeque<Bytes>),
}

fn broken_pipe() -> DynError {
    Box::new(io::Error::from(ErrorKind::BrokenPipe))
}

impl TeeBody {
    pub fn new(body: BoxBody, io: impl AsyncWrite + Send + Sync + 'static) -> Self {
        Self {
            io: Box::pin(io),
            body,
            state: IoState::Writable(VecDeque::with_capacity(2)),
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
                    IoState::Writable(deque) => break 'writable deque,
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

            if let Some(buf) = backlog.front_mut() {
                let bytes_written = match this.io.as_mut().poll_write(context, buf) {
                    Poll::Pending => {
                        return match next {
                            Some(result) => Poll::Ready(Some(result)),
                            None => Poll::Pending,
                        }
                    }
                    Poll::Ready(Ok(n)) => n,
                    Poll::Ready(Err(error)) => {
                        backlog.clear();
                        *state = IoState::Shutdown(Some(error.into()));
                        continue;
                    }
                };

                if bytes_written == 0 && buf.remaining() > 0 {
                    *state = IoState::Shutdown(Some(broken_pipe()));
                    continue;
                }

                if bytes_written < buf.remaining() {
                    buf.advance(bytes_written);
                } else {
                    backlog.pop_front();
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

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body::{Body, Frame};
    use std::collections::VecDeque;
    use std::io;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use tokio::io::AsyncWrite;
    use tokio::sync::Mutex;

    use super::{BoxBody, TeeBody};
    use crate::body::tee_body::IoState;
    use crate::error::DynError;

    struct MockBody {
        ready: bool,
        data: VecDeque<Result<Frame<Bytes>, DynError>>,
    }

    #[derive(Clone)]
    struct MockWriter {
        data: Arc<Mutex<Vec<u8>>>,
    }

    fn io_state_closed(state: &IoState) -> bool {
        match state {
            IoState::Writable(_) | IoState::Shutdown(_) => false,
            IoState::Closed => true,
        }
    }

    fn expect_none(body: Pin<&mut TeeBody>, context: &mut Context) {
        match body.poll_frame(context) {
            Poll::Pending => panic!("expected ready, got pending"),
            Poll::Ready(None) => {}
            Poll::Ready(Some(_)) => panic!("expected none, got some"),
        }
    }

    fn expect_data_frame(body: Pin<&mut TeeBody>, context: &mut Context) -> Bytes {
        match body.poll_frame(context) {
            Poll::Pending => panic!("expected ready, got pending"),
            Poll::Ready(None) => panic!("expected some, got none"),
            Poll::Ready(Some(result)) => result
                .unwrap()
                .into_data()
                .expect("expected data, got trailers"),
        }
    }

    impl MockBody {
        fn new(
            ready: bool,
            data: impl IntoIterator<Item = Result<Frame<Bytes>, DynError>>,
        ) -> Self {
            Self {
                ready,
                data: data.into_iter().collect(),
            }
        }
    }

    impl Body for MockBody {
        type Data = Bytes;
        type Error = DynError;

        fn poll_frame(
            self: Pin<&mut Self>,
            context: &mut Context,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            let this = self.get_mut();

            if this.ready {
                Poll::Ready(this.data.pop_front())
            } else {
                context.waker().wake_by_ref();
                this.ready = true;
                Poll::Pending
            }
        }
    }

    impl MockWriter {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data: Arc::new(Mutex::new(data)),
            }
        }
    }

    impl AsyncWrite for MockWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            context: &mut Context,
            buf: &[u8],
        ) -> Poll<Result<usize, io::Error>> {
            match self.data.try_lock() {
                Ok(mut guard) => {
                    guard.extend_from_slice(buf);
                    Poll::Ready(Ok(buf.len()))
                }
                Err(_) => {
                    context.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            context: &mut Context,
        ) -> Poll<Result<(), io::Error>> {
            eprintln!("poll_shutdown called on MockWriter");
            self.poll_flush(context)
        }
    }

    #[tokio::test]
    async fn test_tee() {
        let writer = MockWriter::new(vec![]);

        let mut context = Context::from_waker(Waker::noop());
        let mut body = TeeBody::new(
            BoxBody::new(MockBody::new(
                true,
                vec![
                    Ok(Frame::data(Bytes::copy_from_slice(b"hello "))),
                    Ok(Frame::data(Bytes::copy_from_slice(b"world"))),
                ],
            )),
            writer.clone(),
        );

        assert_eq!(
            b"hello ".as_slice(),
            expect_data_frame(Pin::new(&mut body), &mut context),
        );

        assert_eq!(
            b"world".as_slice(),
            expect_data_frame(Pin::new(&mut body), &mut context),
        );

        expect_none(Pin::new(&mut body), &mut context);

        assert_eq!(
            b"hello world".as_slice(),
            writer.data.lock().await.as_slice()
        );

        assert!(io_state_closed(&body.state));
    }
}
