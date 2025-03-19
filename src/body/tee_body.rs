use bytes::{Buf, Bytes};
use futures_core::ready;
use http_body::{Body, Frame, SizeHint};
use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

use crate::error::DynError;

/// A boxed body that writes each data frame into a dyn
/// [`AsyncWrite`](AsyncWrite).
///
pub struct TeeBody<T, U> {
    src: T,
    dest: U,
    status: TeeStatus,
    backlog: VecDeque<Bytes>,
}

#[derive(Debug)]
enum TeeStatus {
    Open,
    Closed,
    Pending,
    Shutdown(Option<Result<Frame<Bytes>, DynError>>),
}

fn broken_pipe() -> DynError {
    Box::new(io::Error::from(ErrorKind::BrokenPipe))
}

impl<T, U> TeeBody<T, U>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + Unpin + 'static,
    U: AsyncWrite + Send + Sync + Unpin + 'static,
{
    pub fn new(src: T, dest: U) -> Self {
        Self {
            src,
            dest,
            status: TeeStatus::Pending,
            backlog: VecDeque::with_capacity(2),
        }
    }
}

impl<T, U> Body for TeeBody<T, U>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + Unpin + 'static,
    U: AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let status = &mut this.status;
        let mut done = false;

        loop {
            'shutdown: {
                let last = match status {
                    TeeStatus::Open | TeeStatus::Pending => break 'shutdown,
                    TeeStatus::Shutdown(option) => option.take(),
                    TeeStatus::Closed => return Poll::Ready(None),
                };

                return match Pin::new(&mut this.dest).poll_shutdown(context) {
                    Poll::Pending => {
                        *status = TeeStatus::Shutdown(last);
                        Poll::Pending
                    }
                    Poll::Ready(Ok(())) => {
                        *status = TeeStatus::Closed;
                        Poll::Ready(last)
                    }
                    Poll::Ready(Err(shutdown_error)) => {
                        *status = TeeStatus::Closed;
                        Poll::Ready(last.or_else(|| Some(Err(shutdown_error.into()))))
                    }
                };
            }

            match ready!(Pin::new(&mut this.src).poll_frame(context)) {
                Some(Err(error)) => {
                    this.backlog.clear();
                    *status = TeeStatus::Shutdown(Some(Err(error)));
                    continue;
                }
                Some(Ok(next)) => match next.into_data() {
                    Err(error) => {
                        let frame = Frame::trailers(error.into_trailers().unwrap());
                        *status = TeeStatus::Shutdown(Some(Ok(frame)));
                    }
                    Ok(data) => {
                        this.backlog.push_back(data);
                    }
                },
                None => {
                    done = true;
                }
            }

            if matches!(status, TeeStatus::Pending) {
                *status = TeeStatus::Open;
                context.waker().wake_by_ref();
                return Poll::Pending;
            }

            if let Some(mut front) = this.backlog.pop_front() {
                let remaining = front.remaining();
                let offset = match ready!(Pin::new(&mut this.dest).poll_write(context, &front)) {
                    Ok(num_bytes_written) => num_bytes_written,
                    Err(error) => {
                        this.backlog.clear();
                        *status = TeeStatus::Shutdown(Some(Err(error.into())));
                        continue;
                    }
                };

                if offset == 0 && remaining > 0 {
                    *status = TeeStatus::Shutdown(Some(Err(broken_pipe())));
                    continue;
                }

                if offset < remaining {
                    this.backlog.push_front(front.split_off(offset));
                }

                if done && this.backlog.is_empty() {
                    *status = TeeStatus::Shutdown(Some(Ok(Frame::data(front))));
                    continue;
                }

                return Poll::Ready(Some(Ok(Frame::data(front))));
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(&self.status, TeeStatus::Closed)
    }

    fn size_hint(&self) -> http_body::SizeHint {
        let mut hint = SizeHint::new();

        hint.set_lower(self.src.size_hint().lower());
        hint
    }
}

impl<T, U> Debug for TeeBody<T, U> {
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

    use super::{TeeBody, TeeStatus};
    use crate::error::DynError;

    struct MockBody {
        ready: bool,
        data: VecDeque<Result<Frame<Bytes>, DynError>>,
    }

    #[derive(Clone)]
    struct MockWriter {
        data: Arc<Mutex<Vec<u8>>>,
    }

    fn expect_data_frame(
        body: Pin<&mut TeeBody<MockBody, MockWriter>>,
        context: &mut Context,
    ) -> Bytes {
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
            MockBody::new(
                true,
                vec![
                    Ok(Frame::data(Bytes::copy_from_slice(b"hello "))),
                    Ok(Frame::data(Bytes::copy_from_slice(b"world"))),
                ],
            ),
            writer.clone(),
        );

        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut context),
            Poll::Pending
        ));

        assert_eq!(
            b"hello ".as_slice(),
            expect_data_frame(Pin::new(&mut body), &mut context),
        );

        assert_eq!(
            b"world".as_slice(),
            expect_data_frame(Pin::new(&mut body), &mut context),
        );

        assert_eq!(
            b"hello world".as_slice(),
            writer.data.lock().await.as_slice()
        );

        assert!(
            matches!(&body.status, TeeStatus::Closed),
            "expected TeeStatus::Closed, got {:?}",
            &body.status
        );
    }
}
