use bytes::{Buf, Bytes};
use http::{HeaderMap, StatusCode};
use http_body::Body;
use http_body_util::{LengthLimitError, Limited};
use hyper::body::Incoming;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{Ordering, compiler_fence};
use std::task::{Context, Poll, ready};
use std::{ptr, slice};

use crate::error::{BoxError, Error};
use crate::raise;

/// Interact with data received from a client.
///
pub trait Payload: Sized {
    /// Coalesces the non-contiguous bytes in self into a `Vec<u8>`.
    ///
    /// The buffer that backs each frame of the original payload is zeroized
    /// after the data contained in the frame is read into `dest`.
    ///
    fn coalesce(self) -> Vec<u8>;

    /// Coalesces the non-contiguous bytes in self into a `Vec<u8>`.
    ///
    /// The buffer that backs each frame of the original payload is zeroized
    /// after the data contained in the frame is read into the vec returned by
    /// this function.
    ///
    fn z_coalesce(self) -> Result<Vec<u8>, Self>;

    /// Deserialize and extract `T` as JSON from the top-level data field of
    /// the object contained by the bytes in self.
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use serde::Deserialize;
    /// # use via::Payload;
    /// #
    /// #[derive(Deserialize)]
    /// struct Cat {
    ///     name: String,
    /// }
    ///
    /// let mut payload = Bytes::copy_from_slice(b"{\"data\":{\"name\":\"Ciro\"}}");
    /// let cat = payload.json::<Cat>().expect("invalid payload");
    ///
    /// println!("Meow, {}!", cat.name);
    /// // => Meow, Ciro!
    /// ```
    ///
    fn json<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        deserialize_json(&self.coalesce())
    }

    /// Deserialize and extract `T` as JSON from the top-level data field of
    /// the object contained by the bytes in self. Then, zeroes each frame of
    /// the underlying buffer.
    ///
    fn z_json<T>(self) -> Result<Result<T, Error>, Self>
    where
        T: DeserializeOwned,
    {
        self.z_coalesce().map(|data| deserialize_json(&data))
    }

    fn be_z_json<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.z_json().unwrap_or_else(|payload| {
            // TODO: Placeholder for tracing...
            payload.json()
        })
    }

    /// Copy the bytes in self into an owned, contiguous `String`.
    ///
    /// # Errors
    ///
    /// If the payload is not valid `UTF-8`.
    ///
    fn utf8(self) -> Result<String, Error> {
        let data = self.coalesce();

        String::from_utf8(data).or_else(|error| {
            raise!(400, message = error.to_string());
        })
    }

    fn z_utf8(self) -> Result<Result<String, Error>, Self> {
        self.z_coalesce().map(|data| {
            String::from_utf8(data).or_else(|error| {
                raise!(400, message = error.to_string());
            })
        })
    }

    fn be_z_utf8(self) -> Result<String, Error> {
        self.z_utf8().unwrap_or_else(|payload| {
            // TODO: Placeholder for tracing...
            payload.utf8()
        })
    }
}

/// The data and trailers of a request body.
///
pub struct Aggregate {
    payload: RequestPayload,
    _unsend: PhantomData<Rc<()>>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Coalesce {
    body: Limited<Incoming>,
    payload: Option<RequestPayload>,
}

struct RequestPayload {
    frames: SmallVec<[Bytes; 1]>,
    trailers: Option<HeaderMap>,
}

fn already_read<T>() -> Result<T, Error> {
    raise!(500, message = "The request body has already been read.")
}

#[inline]
fn deserialize_json<T>(buf: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    #[derive(Deserialize)]
    struct Tagged<D> {
        data: D,
    }

    match serde_json::from_slice(buf) {
        Ok(Tagged { data }) => Ok(data),
        Err(error) => raise!(400, error),
    }
}

fn into_future_error<T>(error: BoxError) -> Result<T, Error> {
    if error.is::<LengthLimitError>() {
        // Payload Too Large
        raise!(413, boxed = error);
    }

    // Bad Request
    raise!(400, boxed = error);
}

/// Zeroize the buffer backing the provided `Bytes`. Afterwards, the data in
/// `frame` is unreachable and the visible length is 0.
///
/// To safely call this fn, you must guarantee the following invariants:
///
///   1. `Bytes::is_unique` is true for `frame`
///   2. `compiler_fence` is called after each frame is zeroized
///
/// Adapted from the [zeroize] crate in order to prevent an O(n) call to
/// compiler_fence where n is the number of frames in a payload.
///
/// [zeroize]: https://crates.io/crates/zeroize/
#[inline(never)]
unsafe fn unfenced_zeroize(frame: &mut Bytes) {
    let len = frame.remaining();
    let ptr = frame.as_ptr() as *mut u8;

    for idx in 0..len {
        unsafe {
            ptr::write_volatile(ptr.add(idx), 0);
        }
    }

    // Make the visible length of the frame buffer 0.
    frame.advance(len);
}

#[inline(always)]
fn release_compiler_fence() {
    compiler_fence(Ordering::Release);
}

impl Aggregate {
    fn new(payload: RequestPayload) -> Self {
        Self {
            payload,
            _unsend: PhantomData,
        }
    }

    pub fn len(&self) -> Option<usize> {
        self.payload
            .iter()
            .map(Buf::remaining)
            .try_fold(0usize, |len, remaining| len.checked_add(remaining))
    }

    pub fn is_empty(&self) -> bool {
        self.len().is_some_and(|len| len == 0)
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.payload.trailers.as_ref()
    }

    pub fn mut_trailers(&mut self) -> &mut Option<HeaderMap> {
        &mut self.payload.trailers
    }
}

impl Payload for Aggregate {
    fn coalesce(mut self) -> Vec<u8> {
        let mut dest = Vec::new();

        for frame in self.payload.iter_mut() {
            // The transport layer sufficiently chunks each frame.
            dest.extend_from_slice(frame.as_ref());

            // Make the visible length of the frame buffer 0.
            frame.advance(frame.remaining());
        }

        dest
    }

    fn z_coalesce(mut self) -> Result<Vec<u8>, Self> {
        // If we do not have unique access to each frame in self, return back
        // to the caller.
        if !self.payload.iter().all(Bytes::is_unique) {
            return Err(self);
        }

        let mut dest = Vec::new();

        for frame in self.payload.iter_mut() {
            // The transport layer sufficiently chunks each frame.
            dest.extend_from_slice(frame.as_ref());

            // Safety:
            //
            // The precondition at the top of this function ensures that we
            // have unique access to each frame contained in self.
            //
            // Since Aggregate is also !Send + !Sync, it is impossible to wrap
            // an instance of Aggregate in an Arc and send or share a clone of
            // self with another task.
            //
            // The combination of the aforementioned proofs confirms that we
            // can safely mutate the buffer backing each frame in the payload.
            unsafe {
                unfenced_zeroize(frame);
            }
        }

        // Ensures sequential access to the buffers contained in self.
        // A necessary step after zeroization.
        release_compiler_fence();

        Ok(dest)
    }
}

impl Coalesce {
    pub(super) fn new(body: Limited<Incoming>) -> Self {
        Self {
            body,
            payload: Some(RequestPayload::new()),
        }
    }
}

impl Future for Coalesce {
    type Output = Result<Aggregate, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let Self { body, payload } = self.get_mut();
        let mut body = Pin::new(body);

        loop {
            let Some(result) = ready!(body.as_mut().poll_frame(context)) else {
                return Poll::Ready(match payload.take() {
                    Some(payload) => Ok(Aggregate::new(payload)),
                    None => already_read(),
                });
            };

            let frame = result.or_else(into_future_error)?;
            let payload = payload.as_mut().map_or_else(already_read, Ok)?;
            let trailers = match frame.into_data() {
                Ok(data) => {
                    payload.frames.push(data);
                    continue;
                }
                Err(frame) => {
                    let Ok(trailers) = frame.into_trailers() else {
                        return Poll::Ready(Err(Error::new(
                            StatusCode::BAD_REQUEST,
                            "unexpected frame type received while reading the request body",
                        )));
                    };

                    trailers
                }
            };

            if let Some(existing) = payload.trailers.as_mut() {
                existing.extend(trailers);
            } else {
                payload.trailers = Some(trailers);
            }
        }
    }
}

impl RequestPayload {
    fn new() -> Self {
        Self {
            frames: SmallVec::new(),
            trailers: None,
        }
    }

    #[inline]
    fn iter(&self) -> slice::Iter<'_, Bytes> {
        self.frames.iter()
    }

    #[inline]
    fn iter_mut(&mut self) -> slice::IterMut<'_, Bytes> {
        self.frames.iter_mut()
    }
}

impl Payload for Bytes {
    fn coalesce(mut self) -> Vec<u8> {
        let mut dest = Vec::new();

        // The transport layer sufficiently chunks each frame.
        dest.extend_from_slice(self.as_ref());

        // Make the visible length of the frame buffer 0.
        self.advance(self.remaining());

        dest
    }

    fn z_coalesce(mut self) -> Result<Vec<u8>, Self> {
        // If we do not have unique access to self, return back to the caller.
        if !self.is_unique() {
            return Err(self);
        }

        let mut dest = Vec::new();

        // The transport layer sufficiently chunks each frame.
        dest.extend_from_slice(self.as_ref());

        // Safety:
        //
        // The precondition at the top of this function ensures that we
        // have unique access to self and therefore, can mutate the buffer.
        unsafe {
            unfenced_zeroize(&mut self);
        }

        // Ensures sequential access to the buffers contained in self.
        // A necessary step after zeroization.
        release_compiler_fence();

        Ok(dest)
    }

    fn json<T>(mut self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        // Attempt to deserialize `T` from the bytes in self.
        let result = deserialize_json(self.as_ref());

        // Make the visible length of the frame buffer 0.
        self.advance(self.remaining());

        result
    }

    fn z_json<T>(mut self) -> Result<Result<T, Error>, Self>
    where
        T: DeserializeOwned,
    {
        // If we do not have unique access to self, return back to the caller.
        if !self.is_unique() {
            return Err(self);
        }

        // Attempt to deserialize `T` from the bytes in self.
        let result = deserialize_json(self.as_ref());

        // Safety:
        //
        // The precondition at the top of this function ensures that we
        // have unique access to self and therefore, can mutate the buffer.
        unsafe {
            unfenced_zeroize(&mut self);
        }

        // Ensures sequential access to the buffers contained in self.
        // A necessary step after zeroization.
        release_compiler_fence();

        Ok(result)
    }
}
