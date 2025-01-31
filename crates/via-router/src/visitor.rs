#![allow(clippy::single_match)]

use std::fmt::{self, Display, Formatter};

use crate::Param;

/// The route tree is missing a node that is referenced by another node.
//
// This is an unlikely error that could indicate that the memory where the
// route tree is stored has been corrupted.
//
#[derive(Clone, Debug)]
pub struct RouterError;

/// A matched node in the route tree.
///
/// Contains a reference to the route associated with the node and additional
/// metadata about the match.
///
#[derive(Debug)]
pub struct Found<'a, T> {
    /// True if there were no more segments to match against the children of
    /// the matched node. Otherwise, false.
    ///
    pub exact: bool,

    /// The name of the dynamic parameter that matched the path segment.
    ///
    pub param: Option<&'a Param>,

    /// The start and end offset of the parameter that matched the path
    /// segment.
    ///
    pub range: Option<[usize; 2]>,

    /// The key of the route associated with the node that matched the path
    /// segment.
    ///
    pub route: Option<&'a T>,
}

#[derive(Clone, Debug)]
pub struct Match {
    value: usize,
    range: Option<[usize; 2]>,
}

impl std::error::Error for RouterError {}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "a node was visited that contains an invalid reference")
    }
}

impl Match {
    #[inline]
    pub(crate) fn found(exact: bool, key: usize, range: Option<[usize; 2]>) -> Self {
        Self {
            range,
            value: (key << 2) | (1 << 0) | (if exact { 1 } else { 0 } << 1),
        }
    }

    #[inline]
    pub(crate) fn not_found() -> Self {
        Self {
            value: 0,
            range: None,
        }
    }

    #[inline]
    pub(crate) fn try_load(self) -> Result<(bool, usize, Option<[usize; 2]>), RouterError> {
        let Self { range, value } = self;

        if value & 0b01 != 0 {
            Ok(((value & 0b10) != 0, value >> 2, range))
        } else {
            Err(RouterError)
        }
    }
}
