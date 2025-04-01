//! A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
//
// This code was originally adapted from the `hyper_util::rt::tokio::TokioIo`
// struct in [hyper-util](https://docs.rs/hyper-util).
//

use hyper::rt::{Read, ReadBufCursor, Write};
use std::io::{Error, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A hyper-compatible wrapper for a duplex stream.
///
pub struct IoStream<T> {
    stream: T,
}

impl<T> IoStream<T> {
    #[inline]
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T> IoStream<T> {
    #[inline]
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|this| &mut this.stream) }
    }
}

impl<R: AsyncRead> Read for IoStream<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        mut cursor: ReadBufCursor<'_>,
    ) -> Poll<Result<(), Error>> {
        // Get a tokio-compatible buffer from the unfilled portion of the
        // buffer in ReadBufCursor.
        //
        // Safety:
        //
        // This is safe because we confirm that the cursor is advanced by the
        // the exact number of bytes that were read into the buffer.
        //
        let mut buf = unsafe { ReadBuf::uninit(cursor.as_mut()) };

        let len = match self.project().poll_read(context, &mut buf)? {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(_) => buf.filled().len(),
        };

        // Bytes were read into buf successfully. Advance the cursor by the
        // number of bytes that were read.
        //
        // Safety:
        //
        // This is safe because we are using the exact number of bytes that
        // were read into the filled portion of buf, immediately after the
        // call to poll_read.
        //
        unsafe {
            cursor.advance(len);
        }

        Poll::Ready(Ok(()))
    }
}

impl<W: AsyncWrite> Write for IoStream<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.project().poll_write(context, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().poll_flush(context)
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().poll_shutdown(context)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, Error>> {
        self.project().poll_write_vectored(context, buf_list)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }
}
