use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::Poll;

use super::{Boxed, Buffered};
use crate::{Error, Result};

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

pub enum EitherProjection<'a, L, R> {
    Left(Pin<&'a mut L>),
    Right(Pin<&'a mut R>),
}

impl<L, R> Either<L, R> {
    /// Returns the projection type for the `Either` enum that contains a mutable
    /// reference to the `L` or `R` value that is pinned in memory. This method
    /// is called exclusively in the `poll_frame` method of the implementation of
    /// the `Body` trait for `Self`.
    fn project(self: Pin<&mut Self>) -> EitherProjection<L, R> {
        // Get a mutable reference to `self`.
        let this = unsafe {
            //
            // Safety:
            //
            // The generic types `L` and `R` are not guaranteed to be `Unpin`. We
            // need to get a mutable reference to `self` to project the contained
            // `L` or `R` value. This is safe because no data is moved out of
            // `self`.
            self.get_unchecked_mut()
        };

        match this {
            Either::Left(left) => {
                // Get a pinned reference to the pointer at `left`.
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The `left` value is a mutable reference to `L`. We need
                    // to pin the reference to `L` so that it can be polled in
                    // the `poll_frame` function. This is safe because we are
                    // not moving the value out of the mutable reference.
                    Pin::new_unchecked(left)
                };

                // Return the projection type for the `Left` variant.
                EitherProjection::Left(pin)
            }
            Either::Right(right) => {
                // Get a pinned reference to the pointer at `left`.
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The `right` value is a mutable reference to `R`. We need
                    // to pin the reference to `R` so that it can be polled in
                    // the `poll_frame` function. This is safe because we are
                    // not moving the value out of the mutable reference.
                    Pin::new_unchecked(right)
                };

                // Return the projection type for the `Right` variant.
                EitherProjection::Right(pin)
            }
        }
    }
}

impl<L, R> Body for Either<L, R>
where
    L: Body<Data = Bytes, Error = Error>,
    R: Body<Data = Bytes, Error = Error>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            EitherProjection::Left(left) => left.poll_frame(context),
            EitherProjection::Right(right) => right.poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Either::Left(left) => left.is_end_stream(),
            Either::Right(right) => right.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Either::Left(left) => left.size_hint(),
            Either::Right(right) => right.size_hint(),
        }
    }
}

impl Default for Either<Buffered, Boxed> {
    fn default() -> Self {
        Either::Left(Buffered::default())
    }
}

impl<T> From<T> for Either<Buffered, Boxed>
where
    T: Into<Buffered>,
{
    fn from(value: T) -> Self {
        Either::Left(value.into())
    }
}

impl From<Boxed> for Either<Buffered, Boxed> {
    fn from(body: Boxed) -> Self {
        Either::Right(body)
    }
}
