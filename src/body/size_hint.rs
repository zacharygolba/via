//! Helper functions that convert the size hint of a body to a stream and vice
//! versa.
//!

use http_body::SizeHint;

/// Converts a [`SizeHint`] to a tuple compatible with the
/// [`Stream`](futures_core::Stream)
/// trait.
///
pub fn from_body_for_stream(hint: SizeHint) -> (usize, Option<usize>) {
    // Attempt to convert the lower bound to a usize. If the conversion
    // fails, consider the lower bound to be zero.
    let lower = hint.lower().try_into().unwrap_or(0);
    // Attempt to convert the upper bound to a usize. If the conversion
    // fails, return `None`.
    let upper = hint.upper().and_then(|value| value.try_into().ok());

    // Return the adapted size hint.
    (lower, upper)
}

/// Converts the return value of
/// [`Stream::size_hint`](futures_core::Stream::size_hint)
/// to a
/// [`SizeHint`].
///
pub fn from_stream_for_body((lower, upper): (usize, Option<usize>)) -> SizeHint {
    // Create a new `SizeHint` with the default values. We will set
    // the lower and upper bounds individually since we don't know
    // if the stream has an upper bound.
    let mut hint = SizeHint::new();

    // Attempt to cast the lower bound to a `u64`. If the cast fails,
    // do not set the lower bound of the size hint.
    hint.set_lower(lower.try_into().unwrap_or(0));

    // Check if the upper bound is `Some`. If it is, attempt to cast
    // it to a `u64`. If the cast fails or the upper bound is `None`,
    // do not set the upper bound of the size hint.
    if let Some(Ok(value)) = upper.map(|n| n.try_into()) {
        hint.set_upper(value);
    }

    // Return the adapted size hint.
    hint
}
