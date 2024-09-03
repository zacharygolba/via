use hyper::rt::{Read, ReadBufCursor, Write};
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
pub struct IoStream<T> {
    stream: T,
}

impl<T: Unpin> IoStream<T> {
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T: Unpin> IoStream<T> {
    // Returns a pinned mutable reference to the `stream` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        // Get a mutable refence to self from Pin<&mut Self>.
        let this = self.get_mut();
        // Get a mutable reference to the `stream` field.
        let ptr = &mut this.stream;

        Pin::new(ptr)
    }
}

impl<R> Read for IoStream<R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
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
        // Delegate the reading of bytes to `self.stream`.
        let result = self.project().poll_read(context, &mut tokio_buf);

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
    W: AsyncWrite + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().poll_write(context, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().poll_flush(context)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.project().poll_shutdown(context)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().poll_write_vectored(context, buf_list)
    }
}
