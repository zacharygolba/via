//! A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
//
// This code was originally adapted from the `hyper_util::rt::tokio::TokioIo`
// struct in [hyper-util](https://docs.rs/hyper-util).
//

use hyper::rt::{Read, ReadBufCursor, Write};
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
///
pub struct IoStream<T> {
    stream: T,
}

impl<T> IoStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T: Unpin> IoStream<T> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        // Get a mutable reference to self.
        let this = self.get_mut();
        // Get a mutable reference to the stream field.
        let ptr = &mut this.stream;

        // Return a pinned mutable reference to the stream field.
        Pin::new(ptr)
    }
}

impl<R> Read for IoStream<R>
where
    R: AsyncRead + Send + Sync + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        mut cursor: ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        //
        // Safety:
        //
        // This is safe because we have verified that every permutation of
        // `IoStream` does not uninitialize bytes in the underlying buffer
        // in the the implementation of `R::poll_read`.
        //
        let mut buf = unsafe { ReadBuf::uninit(cursor.as_mut()) };
        let poll = self.project().poll_read(context, &mut buf);

        if let Poll::Ready(Ok(())) = &poll {
            // Get the number of bytes that were read into the uninitialized
            // portion of the buffer.
            let n = buf.filled().len();

            //
            // Safety:
            //
            // This unsafe block is necessary because we need to advance the
            // cursor of `ReadBufCursor` by the number of bytes that were read
            // into the uninitialized portion of the buffer. This is safe because
            // the compiler guarantees that we have unique access to `cursor`
            // within the scope of this function.
            //
            unsafe {
                cursor.advance(n);
            }
        }

        poll
    }
}

impl<W> Write for IoStream<W>
where
    W: AsyncWrite + Send + Sync + Unpin,
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

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().poll_write_vectored(context, buf_list)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }
}
