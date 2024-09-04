use hyper::rt::{Read, ReadBufCursor, Write};
use std::future::Future;
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{Mutex, OwnedMutexGuard};

type IoStreamGuard<T> = OwnedMutexGuard<Pin<Box<T>>>;

/// A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
pub struct IoStream<T> {
    is_write_vectored: bool,
    io_state: IoState<T>,
    stream: Arc<Mutex<Pin<Box<T>>>>,
}

enum IoState<T> {
    Idle,
    Read(Pin<Box<dyn Future<Output = IoStreamGuard<T>> + Send + Sync>>),
    Write(Pin<Box<dyn Future<Output = IoStreamGuard<T>> + Send + Sync>>),
}

/// Attempts to get a new or existing `IoStreamGuard` in a non-blocking manner.
macro_rules! try_lock {
    (
        // Should be an identifier of a variant of the IoState<T> enum. This
        // represents the type of operation that the guard is being acquired
        // for.
        $ty:ident,
        // Should be an identifier of type `Pin<&mut IoStream<T>>`.
        $this:ident,
        // Should be an identifier of type `&mut Context<'_>`.
        $context:ident
    ) => {{
        let mut this = $this;

        loop {
            return match &mut this.io_state {
                IoState::$ty(guard_future) => match Pin::as_mut(guard_future).poll($context) {
                    Poll::Ready(guard) => {
                        this.io_state = IoState::Idle;
                        break guard;
                    }
                    Poll::Pending => Poll::Pending,
                },
                IoState::Idle => {
                    let guard_future = Arc::clone(&this.stream).lock_owned();
                    this.io_state = IoState::$ty(Box::pin(guard_future));
                    continue;
                }
                _ => Poll::Pending,
            };
        }
    }};
}

impl<T> IoStream<T>
where
    T: AsyncRead + AsyncWrite,
{
    pub fn new(stream: T) -> Self {
        Self {
            is_write_vectored: stream.is_write_vectored(),
            io_state: IoState::Idle,
            stream: Arc::new(Mutex::new(Box::pin(stream))),
        }
    }
}

impl<R> Read for IoStream<R>
where
    R: AsyncRead + Send + Sync + 'static,
{
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let mut guard = try_lock!(Read, self, context);
        let mut tokio_buf = unsafe {
            //
            // Safety:
            //
            // This unsafe block is necessary because we need to access the
            // uninitialized portion of `ReadBufCursor`. We do this during the
            // assignment of `tokio_buf`. We must guarantee that we do not
            // uninitialize any bytes that may have been initialized before. This
            // is in part, due to the fact that `IoStream` implements both `Read`
            // and `Write` and we wouldn't want to uninitialized any bytes that
            // may have been initialized by a previous call to `poll_write`.
            //
            ReadBuf::uninit(buf.as_mut())
        };

        let result = guard.as_mut().poll_read(context, &mut tokio_buf);

        if let Poll::Ready(Ok(())) = &result {
            // Get the number of bytes that were read into the uninitialized
            // portion of the buffer.
            let n = tokio_buf.filled().len();

            unsafe {
                //
                // Safety:
                //
                // This unsafe block is necessary because we need to advance
                // the cursor of `ReadBufCursor` by the number of bytes that
                // were read into the uninitialized portion of the buffer. We
                // must guarentee that the length of the filled portion of
                // `tokio_buf` is accurate to uphold the safety of `poll_read`.
                //
                // Heads up for off-by-one errors.
                //
                buf.advance(n);
            }
        }

        result
    }
}

impl<W> Write for IoStream<W>
where
    W: AsyncWrite + Send + Sync + 'static,
{
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let mut guard = try_lock!(Write, self, context);

        guard.as_mut().poll_write(context, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let mut guard = try_lock!(Write, self, context);

        guard.as_mut().poll_flush(context)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let mut guard = try_lock!(Write, self, context);

        guard.as_mut().poll_shutdown(context)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let mut guard = try_lock!(Write, self, context);

        guard.as_mut().poll_write_vectored(context, buf_list)
    }

    fn is_write_vectored(&self) -> bool {
        self.is_write_vectored
    }
}