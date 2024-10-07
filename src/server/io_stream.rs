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
    len: usize,
    stream: T,
}

impl<T> IoStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: T) -> Self {
        Self { len: 0, stream }
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
        let this = self.get_mut();

        //
        // Safety:
        //
        // This is safe because we confirmed that the cursor is advanced by the
        // the exact number of bytes that were read into the buffer.
        //
        let mut buf = unsafe { ReadBuf::uninit(cursor.as_mut()) };

        match Pin::new(&mut this.stream).poll_read(context, &mut buf) {
            Poll::Ready(Ok(())) => {
                // Get the number of bytes that were read into the uninitialized
                // portion of the buffer.
                let n = buf.filled().len();

                // Update the total number of bytes that have been read. Confirm
                // that the cursor is not advanced beyond `usize::MAX`.
                this.len = this.len.checked_add(n).expect("overflow");

                //
                // Safety:
                //
                // This is safe because we haved confirmed that we are advancing the
                // cursor by the number of bytes that were read into the buffer. We
                // also perform a bounds check to ensure that the cursor is not
                // advanced beyond usize::MAX.
                //
                unsafe {
                    cursor.advance(n);
                }

                Poll::Ready(Ok(()))
            }
            result => result,
        }
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
        let this = self.get_mut();
        let stream = &mut this.stream;

        Pin::new(stream).poll_write(context, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.get_mut();
        let stream = &mut this.stream;

        Pin::new(stream).poll_flush(context)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let this = self.get_mut();
        let stream = &mut this.stream;

        Pin::new(stream).poll_shutdown(context)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.get_mut();
        let stream = &mut this.stream;

        Pin::new(stream).poll_write_vectored(context, buf_list)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }
}
