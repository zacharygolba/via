use hyper::rt::{Read, ReadBufCursor, Write};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::OwnedSemaphorePermit;

pub(crate) struct IoWithPermit<T> {
    io: T,
    _permit: OwnedSemaphorePermit,
}

impl<T> IoWithPermit<T> {
    #[inline]
    pub fn new(io: T, _permit: OwnedSemaphorePermit) -> Self {
        Self { io, _permit }
    }
}

impl<T> IoWithPermit<T> {
    #[inline(always)]
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        // Safety: A pin projection.
        unsafe { Pin::map_unchecked_mut(self, |it| &mut it.io) }
    }
}

impl<T: Read + Unpin> Read for IoWithPermit<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: ReadBufCursor,
    ) -> Poll<io::Result<()>> {
        self.project().poll_read(context, buf)
    }
}

impl<T: Write + Unpin> Write for IoWithPermit<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.project().poll_write(context, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().poll_flush(context)
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().poll_shutdown(context)
    }

    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.project().poll_write_vectored(context, bufs)
    }
}
