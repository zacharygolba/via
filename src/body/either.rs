use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use std::{pin::Pin, task::Poll};

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
    pub fn project(self: Pin<&mut Self>) -> EitherProjection<L, R> {
        let this = unsafe {
            // Safety:
            // TODO: Add safety explanation.
            self.get_unchecked_mut()
        };

        match this {
            Either::Left(left) => {
                let pin = unsafe {
                    // Safety:
                    // TODO: Add safety explanation.
                    Pin::new_unchecked(left)
                };

                EitherProjection::Left(pin)
            }
            Either::Right(right) => {
                let pin = unsafe {
                    // Safety:
                    // TODO: Add safety explanation.
                    Pin::new_unchecked(right)
                };

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
