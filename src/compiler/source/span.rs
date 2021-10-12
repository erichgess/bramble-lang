//! The `Span` represents a subset of the source code and is used to link
//! IR elements generated by the compiler to the code that they represent.

use super::Offset;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    /// The span starts at this position in the global source map
    low: Offset,

    /// The span goes upto, but does not include, this offset
    high: Offset,
}

impl Span {
    pub fn new(low: Offset, high: Offset) -> Span {
        if low > high {
            panic!(
                "Cannot have a low that is greater than the high: {:?} and {:?}",
                low, high
            )
        }

        Span { low, high }
    }

    pub fn zero() -> Span {
        Span::new(Offset(0), Offset(1))
    }
}
