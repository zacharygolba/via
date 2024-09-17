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
use tokio::sync::{Mutex, MutexGuard, OwnedMutexGuard};

/// A wrapper around a stream that implements both `AsyncRead` and `AsyncWrite`.
pub struct IoStream<T> {
    /// A flag indicating whether the underlying stream supports vectored writes.
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
    Read(Pin<Box<dyn Future<Output = OwnedMutexGuard<T>> + Send + Sync>>),

    /// The stream will be used for a write operation. Read operations will have
    /// to wait until the write operation is complete before acquiring a lock on
    /// the stream.
    Write(Pin<Box<dyn Future<Output = OwnedMutexGuard<T>> + Send + Sync>>),
}

/// Either a borrowed or owned lock on an I/O stream.
enum IoStreamGuard<'a, T> {
    Borrowed(MutexGuard<'a, T>),
    Owned(OwnedMutexGuard<T>),
}

/// Attempts to get a new or existing `IoStreamGuard` in a non-blocking manner.
macro_rules! try_lock {
    (
        // Should be an identifier of a variant of the IoState<T> enum. This
        // represents the type of operation that the guard is being acquired
        // for.
        $ty:ident,
        // Should be an expression of type `Pin<&mut IoStream<T>>`.
        $self:expr,
        // Should be an expression of type `&mut Context<'_>`.
        $context:expr
    ) => {{
        use self::{IoState, IoStreamGuard};
        use std::task::Poll;

        let context = $context;

        loop {
            // Acquire a lock on the stream for the operation of type `$ty`.
            return match &mut $self.io_state {
                // The stream is idle. We can acquire a lock for the operation
                // of type `$ty`.
                IoState::Idle => {
                    let stream = &mut $self.stream;

                    // Attempt to acquire a lock on the stream. If the lock is
                    // immediately available, we'll get the guard and return it.
                    //
                    // This is the happy path that we expect to take most of the
                    // time. We're able to acquire the lock without incrementing
                    // the reference count of the `Arc` that wraps the stream or
                    // performing any heap allocations.
                    if let Ok(guard) = stream.try_lock() {
                        break IoStreamGuard::Borrowed(guard);
                    }

                    if cfg!(debug_assertions) {
                        //
                        // TODO:
                        //
                        // Replace this with tracing.
                        //
                        eprintln!(
                            "Lock on IoStream was not immediately available. {}",
                            "Falling back to a future."
                        );
                    }

                    // Get a future that resolves to an `OwnedMutexGuard` that
                    // ensures exclusive access to the underlying stream for
                    // the operation of type `$ty`.
                    //
                    // We use an `Arc` to clone the stream so we can move the
                    // stream into the future and remain compatible with multi-
                    // threaded runtimes.
                    let future = Box::pin(stream.clone().lock_owned());

                    // Transition `io_state` to `$ty`. This indicates that we're
                    // waiting for a lock to be acquired before we can proceed
                    // with the intended operation.
                    $self.io_state = IoState::$ty(future);

                    // Continue to the next iteration of the loop to poll the
                    // future we just created.
                    continue;
                }

                // We're currently waiting for a lock to be acquired for the
                // operation of type `$ty`. Poll the future to see if the lock
                // has been acquired.
                IoState::$ty(future) => match future.as_mut().poll(context) {
                    // The lock has been acquired.
                    Poll::Ready(guard) => {
                        // Transition `io_state` to `Idle`. This indicates that
                        // we'll be ready to schedule another operation when
                        // the guard is dropped.
                        $self.io_state = IoState::Idle;

                        // Break out of the loop and return the guard.
                        break IoStreamGuard::Owned(guard);
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
                _ => Poll::Pending,
            };
        }
    }};
}

impl<T> IoStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: T) -> Self {
        let is_write_vectored = stream.is_write_vectored();

        Self {
            stream: Arc::new(Mutex::new(stream)),
            io_state: IoState::Idle,
            is_write_vectored,
        }
    }
}

impl<R> Read for IoStream<R>
where
    R: AsyncRead + Send + Sync + Unpin + 'static,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
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

        let result = match try_lock!(Read, self, &mut *context) {
            IoStreamGuard::Borrowed(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_read(context, &mut buf)
            }
            IoStreamGuard::Owned(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_read(context, &mut buf)
            }
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
    W: AsyncWrite + Send + Sync + Unpin + 'static,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match try_lock!(Write, self, &mut *context) {
            IoStreamGuard::Borrowed(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_write(context, buf)
            }
            IoStreamGuard::Owned(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_write(context, buf)
            }
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match try_lock!(Write, self, &mut *context) {
            IoStreamGuard::Borrowed(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_flush(context)
            }
            IoStreamGuard::Owned(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_flush(context)
            }
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match try_lock!(Write, self, &mut *context) {
            IoStreamGuard::Borrowed(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_shutdown(context)
            }
            IoStreamGuard::Owned(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_shutdown(context)
            }
        }
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf_list: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        match try_lock!(Write, self, &mut *context) {
            IoStreamGuard::Borrowed(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_write_vectored(context, buf_list)
            }
            IoStreamGuard::Owned(mut guard) => {
                let ptr = &mut *guard;
                let stream = Pin::new(ptr);

                stream.poll_write_vectored(context, buf_list)
            }
        }
    }

    fn is_write_vectored(&self) -> bool {
        self.is_write_vectored
    }
}
