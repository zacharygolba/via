#![allow(clippy::single_match)]

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::Param;

#[derive(Clone, Debug)]
pub enum VisitError {
    /// The route tree is missing a node that is referenced by another node.
    //
    // This is an unlikely error that could indicate that the memory where the
    // route tree is stored has been corrupted.
    //
    NodeNotFound,

    /// The route tree is missing the root node.
    //
    // This is a *very* unlikely error that could indicate that the memory where
    // the route tree is stored has been corrupted.
    //
    RootNotFound,
}

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

impl Match {
    #[inline]
    pub(crate) fn new(exact: bool, key: usize, range: Option<[usize; 2]>) -> Self {
        Self {
            range,
            value: (key << 2) | (1 << 0) | (if exact { 1 } else { 0 } << 1),
        }
    }

    #[inline]
    pub(crate) fn try_load(self) -> Result<(bool, usize, Option<[usize; 2]>), VisitError> {
        let Match { range, value } = self;

        if value & 0b01 == 0 {
            Err(VisitError::NodeNotFound)
        } else {
            Ok(((value & 0b10) != 0, value >> 2, range))
        }
    }
}

impl Default for Match {
    #[inline]
    fn default() -> Self {
        Self {
            value: 0,
            range: None,
        }
    }
}

impl Error for VisitError {}

impl Display for VisitError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::NodeNotFound => {
                write!(f, "a node was visited that contains an invalid reference")
            }
            Self::RootNotFound => {
                write!(f, "the route tree is missing the root node")
            }
        }
    }
}
