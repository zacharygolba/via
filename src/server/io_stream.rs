//! A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
//
// This code was originally adapted from the `hyper_util::rt::tokio::TokioIo`
// struct in [hyper-util](https://docs.rs/hyper-util).
//

use hyper::rt::{Read, ReadBufCursor, Write};
use std::future::Future;
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{Mutex, OwnedMutexGuard};

/// A type alias for the mutex guard around the stream field of IoStream.
type IoStreamGuard<T> = OwnedMutexGuard<T>;

/// A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
pub struct IoStream<T> {
    /// A cached of whether `T` supports vectored writes. We store this value
    /// before `T` is wrapped in a Mutex to avoid the overhead of having to
    /// acquire a lock every time we need to check if vectored writes are
    /// supported.
    ///
    /// This is safe because we only ever wrap TcpStream in an IoStream and
    /// TcpStream returns a constant value for `is_write_vectored`. If we ever
    /// support other types that have a dynamic value for `is_write_vectored`, we
    /// will need to change our approach.
    is_write_vectored: bool,

    /// The underlying I/O stream that we're wrapping. Currently, `T` is always
    /// a `TcpStream`. When we add support HTTPS, HTTP/2, or alternative async
    /// runtime implementations there will be additional permutations of `T`.
    stream: Arc<Mutex<T>>,

    /// Represents the pending state of an I/O operation that is waiting for a
    /// lock to be acquired on `self.stream`.
    io_state: IoState<T>,
}

/// A wrapper around a `Future` that resolves to an `OwnedMutexGuard` that is
/// to ensure exclusive access to the underlying stream for a read or write
/// operation.
enum IoState<T> {
    /// The stream is idle. This means that a read or write operation can be
    /// scheduled by acquiring a lock on the stream.
    Idle,

    /// The stream will be used for a read operation. Write operations will have
    /// to wait until the read operation is complete before acquiring a lock on
    /// the stream.
    Read(Pin<Box<dyn Future<Output = IoStreamGuard<T>> + Send + Sync>>),

    /// The stream will be used for a write operation. Read operations will have
    /// to wait until the write operation is complete before acquiring a lock on
    /// the stream.
    Write(Pin<Box<dyn Future<Output = IoStreamGuard<T>> + Send + Sync>>),
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
        // Should be an expression of type `&mut IoStream<T>`.
        $this:expr,
        // Should be an expression of type `&mut Context<'_>`.
        $context:expr
    ) => {{
        let this = $this;
        let context = $context;

        loop {
            // Acquire a lock on the stream for the operation of type `$ty`.
            return match &mut this.io_state {
                // The stream is idle. We can acquire a lock for the operation
                // of type `$ty`.
                IoState::Idle => {
                    // Get a future that resolves to an `OwnedMutexGuard` that
                    // ensures exclusive access to the underlying stream for
                    // the operation of type `$ty`.
                    //
                    // We use an `Arc` to clone the stream so we can move the
                    // stream into the future and remain compatible with multi-
                    // threaded runtimes.
                    let future = Box::pin(Arc::clone(&this.stream).lock_owned());

                    // Transition `io_state` to `$ty`. This indicates that we're
                    // waiting for a lock to be acquired before we can proceed
                    // with the intended operation.
                    this.io_state = IoState::$ty(future);

                    // Continue to the next iteration of the loop to poll the
                    // future we just created.
                    continue;
                }

                // We're currently waiting for a lock to be acquired for the
                // operation of type `$ty`. Poll the future to see if the lock
                // has been acquired.
                IoState::$ty(future) => match Pin::as_mut(future).poll(context) {
                    // The lock has been acquired.
                    Poll::Ready(guard) => {
                        // Transition `io_state` to `Idle`. This indicates that
                        // we'll be ready to schedule another operation when
                        // the guard is dropped.
                        this.io_state = IoState::Idle;

                        // Break out of the loop and return the guard.
                        break guard;
                    }

                    // The lock has not been acquired yet.
                    Poll::Pending => {
                        // Return `Poll::Pending`. We'll be woken up when the
                        // lock is acquired.
                        Poll::Pending
                    }
                },

                // The stream is currently being used for a different operation.
                // We'll have to wait until the stream is idle before we can
                // schedule the intended read or write operation. This is a
                // very unlikely sceanrio to occur but we handle it to guarantee
                // the API contract of the unsafe blocks in `poll_read`.
                _ => {
                    // Wake the current task so it can be scheduled again. We
                    // do this because we don't want to poll the future and
                    // inadvertently steal the guard from an operation that is
                    // different from what we intended.
                    context.waker().wake_by_ref();

                    // Return `Poll::Pending`.
                    Poll::Pending
                }
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
            stream: Arc::new(Mutex::new(stream)),
            io_state: IoState::Idle,
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
        mut cursor: ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let mut buf = unsafe {
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
            ReadBuf::uninit(cursor.as_mut())
        };

        let result = {
            let this = self.get_mut();
            let mut guard = try_lock!(Read, this, &mut *context);

            AsyncRead::poll_read(Pin::new(&mut *guard), context, &mut buf)
        };

        if let Poll::Ready(Ok(())) = &result {
            // Get the number of bytes that were read into the uninitialized
            // portion of the buffer.
            let n = buf.filled().len();

            unsafe {
                //
                // Safety:
                //
                // This unsafe block is necessary because we need to advance the
                // cursor of `ReadBufCursor` by the number of bytes that were
                // read into the uninitialized portion of the buffer.
                //
                cursor.advance(n);
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
        let mut guard = {
            let this = self.get_mut();
            try_lock!(Write, this, &mut *context)
        };

        AsyncWrite::poll_write(Pin::new(&mut *guard), context, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let mut guard = {
            let this = self.get_mut();
            try_lock!(Write, this, &mut *context)
        };

        AsyncWrite::poll_flush(Pin::new(&mut *guard), context)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let mut guard = {
            let this = self.get_mut();
            try_lock!(Write, this, &mut *context)
        };

        AsyncWrite::poll_shutdown(Pin::new(&mut *guard), context)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let mut guard = {
            let this = self.get_mut();
            try_lock!(Write, this, &mut *context)
        };

        AsyncWrite::poll_write_vectored(Pin::new(&mut *guard), context, buf_list)
    }

    fn is_write_vectored(&self) -> bool {
        self.is_write_vectored
    }
}
