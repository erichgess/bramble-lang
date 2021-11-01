//! The `Span` represents a subset of the source code and is used to link
//! IR elements generated by the compiler to the code that they represent.

use super::Offset;

/// Trait that any IR type which derives from or represents source code
/// must implement.  This trait contains functions for getting the [`Span`]
/// of source code that an IR value represents, covers, or models.
pub trait HasSpan {
    /// Get the [`Span`] of source code that this value covers
    fn span(&self) -> Span;
}

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

    /// Returns the lower [`Offset`] of this Span
    pub fn low(&self) -> Offset {
        self.low
    }

    /// Returns the upper bound [`Offset`] of this span
    pub fn high(&self) -> Offset {
        self.high
    }

    pub fn zero() -> Span {
        Span::new(Offset(0), Offset(0))
    }

    /// Creates the smallest span that covers the two given spans.
    pub fn cover(a: Span, b: Span) -> Span {
        let low = a.low.min(b.low);
        let high = a.high.max(b.high);

        Span::new(low, high)
    }

    /// Returns true if this [`Span`] and the Span `b` intersect
    pub fn intersects(&self, b: Span) -> bool {
        self.intersection(b).is_some()
    }

    /// If this and `a` intersect, then this will return the [`Span`] that
    /// covers that intersection.
    pub fn intersection(&self, b: Span) -> Option<Span> {
        // Test for the intersection of self and b
        let low = if self.low < b.low { b.low } else { self.low };
        let high = if self.high < b.high {
            self.high
        } else {
            b.high
        };

        if low < high {
            Some(Span::new(low, high))
        } else {
            None
        }
    }
}
