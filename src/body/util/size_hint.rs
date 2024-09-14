//! Conversion methods to go to and from the size hint of a body or stream.
//!

use futures::Stream;
use http_body::{Body, SizeHint};

/// Adapts the `SizeHint` of a `Body` to a tuple containing the lower and upper
/// bound of the stream. If the conversion of the lower bound fails, it will be
/// considered to be `0`. If the conversion of the upper bound fails, it will
/// be considered to be `None`.
pub fn from_body_for_stream(body: &impl Body) -> (usize, Option<usize>) {
    // Get the size hint from the body stream. We'll use this convert the
    // `SizeHint` type to a tuple containing the upper and lower bound of
    // the stream.
    let hint = body.size_hint();
    // Attempt to convert the lower bound to a usize. If the conversion
    // fails, consider the lower bound to be zero.
    let lower = usize::try_from(hint.lower()).unwrap_or(0);
    // Attempt to convert the upper bound to a usize. If the conversion
    // fails, return `None`.
    let upper = hint.upper().and_then(|value| usize::try_from(value).ok());

    // Return the adapted size hint.
    (lower, upper)
}

/// Adapts the tuple returned from `Stream::size_hint` to a `SizeHint` that is
/// compatible with the `Body` trait. If the conversion of the lower bound fails,
/// it will be considered to be `0`. If the conversion of the upper bound fails,
/// it will be considered to be `None`.
pub fn from_stream_for_body(stream: &(impl Stream + ?Sized)) -> SizeHint {
    // Create a new `SizeHint` with the default values. We will set
    // the lower and upper bounds individually since we don't know
    // if the stream has an upper bound.
    let mut hint = SizeHint::new();

    // Get a tuple containing the lower and upper bounds of the
    // stream. The `BodyTrait` uses the `SizeHint` type rather than
    // a tuple so we need to convert the returned tuple instead of
    // simply delegating the call to the stream.
    let (lower, upper) = stream.size_hint();

    // Attempt to cast the lower bound to a `u64`. If the cast fails,
    // do not set the lower bound of the size hint.
    hint.set_lower(u64::try_from(lower).unwrap_or(0));

    // Check if the upper bound is `Some`. If it is, attempt to cast
    // it to a `u64`. If the cast fails or the upper bound is `None`,
    // do not set the upper bound of the size hint.
    if let Some(Ok(value)) = upper.map(u64::try_from) {
        hint.set_upper(value);
    }

    // Return the adapted size hint.
    hint
}
