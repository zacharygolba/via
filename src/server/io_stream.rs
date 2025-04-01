//! A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
//
// This code was originally adapted from the `hyper_util::rt::tokio::TokioIo`
// struct in [hyper-util](https://docs.rs/hyper-util).
//

use futures_core::ready;
use hyper::rt::{Read, ReadBufCursor, Write};
use std::io::{Error, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A hyper-compatible wrapper for a duplex stream.
///
pub struct IoStream<T> {
    stream: Pin<Box<T>>,
}

impl<T> IoStream<T> {
    #[inline]
    pub fn new(stream: T) -> Self {
        Self {
            stream: Box::pin(stream),
        }
    }
}

impl<R: AsyncRead> Read for IoStream<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
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

        ready!(self.stream.as_mut().poll_read(context, &mut buf)?);
        let len = buf.filled().len();

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
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.stream.as_mut().poll_write(context, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.stream.as_mut().poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        self.stream.as_mut().poll_shutdown(context)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, Error>> {
        self.stream.as_mut().poll_write_vectored(context, buf_list)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }
}
