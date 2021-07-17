// SPDX-License-Identifier: MPL-2.0

//! A term is the fundamental unit of operation of the PubGrub algorithm.
//! It is a positive or negative expression regarding a set of versions.

use crate::range_trait::Range;
use crate::version_trait::{Interval, Version};
use std::fmt;

///  A positive or negative expression regarding a set of versions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Term<I: Interval<V>, V: Version> {
    /// For example, "1.0.0 <= v < 2.0.0" is a positive expression
    /// that is evaluated true if a version is selected
    /// and comprised between version 1.0.0 and version 2.0.0.
    Positive(Range<I, V>),
    /// The term "not v < 3.0.0" is a negative expression
    /// that is evaluated true if a version is selected >= 3.0.0
    /// or if no version is selected at all.
    Negative(Range<I, V>),
}

/// Base methods.
impl<I: Interval<V>, V: Version> Term<I, V> {
    /// A term that is always true.
    pub(crate) fn any() -> Self {
        Self::Negative(Range::empty())
    }

    /// A term that is never true.
    pub(crate) fn empty() -> Self {
        Self::Positive(Range::empty())
    }

    /// A positive term containing exactly that version.
    pub(crate) fn exact(version: V) -> Self {
        Self::Positive(Range::singleton(version))
    }

    /// Simply check if a term is positive.
    pub(crate) fn is_positive(&self) -> bool {
        match self {
            Self::Positive(_) => true,
            Self::Negative(_) => false,
        }
    }

    /// Negate a term.
    /// Evaluation of a negated term always returns
    /// the opposite of the evaluation of the original one.
    pub(crate) fn negate(&self) -> Self {
        match self {
            Self::Positive(range) => Self::Negative(range.clone()),
            Self::Negative(range) => Self::Positive(range.clone()),
        }
    }

    /// Evaluate a term regarding a given choice of version.
    pub(crate) fn contains(&self, v: &V) -> bool {
        match self {
            Self::Positive(range) => range.contains(v),
            Self::Negative(range) => !(range.contains(v)),
        }
    }

    /// Unwrap the range contains in a positive term.
    /// Will panic if used on a negative range.
    pub(crate) fn unwrap_positive(&self) -> &Range<I, V> {
        match self {
            Self::Positive(range) => range,
            _ => panic!("Negative term cannot unwrap positive range"),
        }
    }
}

/// Set operations with terms.
impl<I: Interval<V>, V: Version> Term<I, V> {
    /// Compute the intersection of two terms.
    /// If at least one term is positive, the intersection is also positive.
    pub(crate) fn intersection(&self, other: &Term<I, V>) -> Term<I, V> {
        match (self, other) {
            (Self::Positive(r1), Self::Positive(r2)) => Self::Positive(r1.intersection(r2)),
            (Self::Positive(r1), Self::Negative(r2)) => {
                Self::Positive(r1.intersection(&r2.complement()))
            }
            (Self::Negative(r1), Self::Positive(r2)) => {
                Self::Positive(r1.complement().intersection(r2))
            }
            (Self::Negative(r1), Self::Negative(r2)) => Self::Negative(r1.union(r2)),
        }
    }

    /// Compute the union of two terms.
    /// If at least one term is negative, the union is also negative.
    pub(crate) fn union(&self, other: &Term<I, V>) -> Term<I, V> {
        (self.negate().intersection(&other.negate())).negate()
    }

    /// Indicate if this term is a subset of another term.
    /// Just like for sets, we say that t1 is a subset of t2
    /// if and only if t1 ∩ t2 = t1.
    pub(crate) fn subset_of(&self, other: &Term<I, V>) -> bool {
        self == &self.intersection(other)
    }
}

/// Describe a relation between a set of terms S and another term t.
///
/// As a shorthand, we say that a term v
/// satisfies or contradicts a term t if {v} satisfies or contradicts it.
pub(crate) enum Relation {
    /// We say that a set of terms S "satisfies" a term t
    /// if t must be true whenever every term in S is true.
    Satisfied,
    /// Conversely, S "contradicts" t if t must be false
    /// whenever every term in S is true.
    Contradicted,
    /// If neither of these is true we say that S is "inconclusive" for t.
    Inconclusive,
}

/// Relation between terms.
impl<'a, I: Interval<V>, V: 'a + Version> Term<I, V> {
    /// Check if a set of terms satisfies this term.
    ///
    /// We say that a set of terms S "satisfies" a term t
    /// if t must be true whenever every term in S is true.
    ///
    /// It turns out that this can also be expressed with set operations:
    ///    S satisfies t if and only if  ⋂ S ⊆ t
    #[cfg(test)]
    fn satisfied_by(&self, terms_intersection: &Term<I, V>) -> bool {
        terms_intersection.subset_of(self)
    }

    /// Check if a set of terms contradicts this term.
    ///
    /// We say that a set of terms S "contradicts" a term t
    /// if t must be false whenever every term in S is true.
    ///
    /// It turns out that this can also be expressed with set operations:
    ///    S contradicts t if and only if ⋂ S is disjoint with t
    ///    S contradicts t if and only if  (⋂ S) ⋂ t = ∅
    #[cfg(test)]
    fn contradicted_by(&self, terms_intersection: &Term<I, V>) -> bool {
        terms_intersection.intersection(self) == Self::empty()
    }

    /// Check if a set of terms satisfies or contradicts a given term.
    /// Otherwise the relation is inconclusive.
    pub(crate) fn relation_with(&self, other_terms_intersection: &Term<I, V>) -> Relation {
        let full_intersection = self.intersection(other_terms_intersection);
        if &full_intersection == other_terms_intersection {
            Relation::Satisfied
        } else if full_intersection == Self::empty() {
            Relation::Contradicted
        } else {
            Relation::Inconclusive
        }
    }
}

impl<I: Interval<V>, V: Version> AsRef<Term<I, V>> for Term<I, V> {
    fn as_ref(&self) -> &Term<I, V> {
        &self
    }
}

// REPORT ######################################################################

impl<I: Interval<V>, V: Version + fmt::Display> fmt::Display for Term<I, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Positive(range) => write!(f, "{}", range),
            Self::Negative(range) => write!(f, "Not ( {} )", range),
        }
    }
}

// TESTS #######################################################################

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::version_trait::{NumberInterval, NumberVersion};
    use proptest::prelude::*;

    pub fn strategy() -> impl Strategy<Value = Term<NumberInterval, NumberVersion>> {
        prop_oneof![
            crate::range_trait::tests::strategy().prop_map(Term::Positive),
            crate::range_trait::tests::strategy().prop_map(Term::Negative),
        ]
    }

    proptest! {

        // Testing relation --------------------------------

        #[test]
        fn relation_with(term1 in strategy(), term2 in strategy()) {
            match term1.relation_with(&term2) {
                Relation::Satisfied => assert!(term1.satisfied_by(&term2)),
                Relation::Contradicted => assert!(term1.contradicted_by(&term2)),
                Relation::Inconclusive => {
                    assert!(!term1.satisfied_by(&term2));
                    assert!(!term1.contradicted_by(&term2));
                }
            }
        }

    }
}
