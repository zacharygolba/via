use hyper::rt::{Read, Write};
use hyper_util::rt::TokioIo;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::OwnedSemaphorePermit;

pub(crate) struct IoWithPermit<T> {
    _permit: OwnedSemaphorePermit,
    io: TokioIo<T>,
}

impl<T> IoWithPermit<T> {
    #[inline]
    pub fn new(permit: OwnedSemaphorePermit, io: T) -> Self {
        Self {
            _permit: permit,
            io: TokioIo::new(io),
        }
    }
}

impl<T> IoWithPermit<T> {
    #[inline]
    fn project(self: Pin<&mut Self>) -> Pin<&mut TokioIo<T>> {
        unsafe { self.map_unchecked_mut(|it| &mut it.io) }
    }
}

impl<T: AsyncRead> Read for IoWithPermit<T> {
    #[inline(always)]
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Read::poll_read(self.project(), context, buf)
    }
}

impl<T: AsyncWrite> Write for IoWithPermit<T> {
    #[inline(always)]
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Write::poll_write(self.project(), context, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Write::poll_flush(self.project(), context)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Write::poll_shutdown(self.project(), context)
    }

    fn is_write_vectored(&self) -> bool {
        Write::is_write_vectored(&self.io)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        Write::poll_write_vectored(self.project(), context, bufs)
    }
}
