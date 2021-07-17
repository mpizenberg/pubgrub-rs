// SPDX-License-Identifier: MPL-2.0

//! Traits and implementations to create and compare versions.

use std::fmt::{self, Debug, Display};
use std::ops::{Bound, RangeBounds};
use std::str::FromStr;
use thiserror::Error;

/// New trait for versions.
pub trait Version: Clone + Ord + Debug + Display {
    /// Returns the minimum version.
    fn minimum() -> Bound<Self>;
    /// Returns the maximum version.
    fn maximum() -> Bound<Self>;
}

/// An interval is a bounded domain containing all values
/// between its starting and ending bounds.
pub trait Interval<V>: RangeBounds<V> + Debug + Clone + Eq + PartialEq {
    /// Create an interval from its starting and ending bounds.
    /// It's the caller responsability to order them correctly.
    fn new(start_bound: Bound<V>, end_bound: Bound<V>) -> Self;
}

// SemanticVersion #############################################################

/// Type for semantic versions: major.minor.patch.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SemanticVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

#[cfg(feature = "serde")]
impl serde::Serialize for SemanticVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SemanticVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// Constructors
impl SemanticVersion {
    /// Create a version with "major", "minor" and "patch" values.
    /// `version = major.minor.patch`
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Version 0.0.0.
    pub fn zero() -> Self {
        Self::new(0, 0, 0)
    }

    /// Version 1.0.0.
    pub fn one() -> Self {
        Self::new(1, 0, 0)
    }

    /// Version 2.0.0.
    pub fn two() -> Self {
        Self::new(2, 0, 0)
    }
}

// Convert a tuple (major, minor, patch) into a version.
impl From<(u32, u32, u32)> for SemanticVersion {
    fn from(tuple: (u32, u32, u32)) -> Self {
        let (major, minor, patch) = tuple;
        Self::new(major, minor, patch)
    }
}

// Convert a version into a tuple (major, minor, patch).
impl From<SemanticVersion> for (u32, u32, u32) {
    fn from(v: SemanticVersion) -> Self {
        (v.major, v.minor, v.patch)
    }
}

// Bump versions.
impl SemanticVersion {
    /// Bump the patch number of a version.
    pub fn bump_patch(self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    /// Bump the minor number of a version.
    pub fn bump_minor(self) -> Self {
        Self::new(self.major, self.minor + 1, 0)
    }

    /// Bump the major number of a version.
    pub fn bump_major(self) -> Self {
        Self::new(self.major + 1, 0, 0)
    }
}

/// Error creating [SemanticVersion] from [String].
#[derive(Error, Debug, PartialEq)]
pub enum VersionParseError {
    /// [SemanticVersion] must contain major, minor, patch versions.
    #[error("version {full_version} must contain 3 numbers separated by dot")]
    NotThreeParts {
        /// [SemanticVersion] that was being parsed.
        full_version: String,
    },
    /// Wrapper around [ParseIntError](core::num::ParseIntError).
    #[error("cannot parse '{version_part}' in '{full_version}' as u32: {parse_error}")]
    ParseIntError {
        /// [SemanticVersion] that was being parsed.
        full_version: String,
        /// A version part where parsing failed.
        version_part: String,
        /// A specific error resulted from parsing a part of the version as [u32].
        parse_error: String,
    },
}

impl FromStr for SemanticVersion {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse_u32 = |part: &str| {
            part.parse::<u32>().map_err(|e| Self::Err::ParseIntError {
                full_version: s.to_string(),
                version_part: part.to_string(),
                parse_error: e.to_string(),
            })
        };

        let mut parts = s.split('.');
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(major), Some(minor), Some(patch), None) => {
                let major = parse_u32(major)?;
                let minor = parse_u32(minor)?;
                let patch = parse_u32(patch)?;
                Ok(Self {
                    major,
                    minor,
                    patch,
                })
            }
            _ => Err(Self::Err::NotThreeParts {
                full_version: s.to_string(),
            }),
        }
    }
}

#[test]
fn from_str_for_semantic_version() {
    let parse = |str: &str| str.parse::<SemanticVersion>();
    assert!(parse(
        &SemanticVersion {
            major: 0,
            minor: 1,
            patch: 0
        }
        .to_string()
    )
    .is_ok());
    assert!(parse("1.2.3").is_ok());
    assert_eq!(
        parse("1.abc.3"),
        Err(VersionParseError::ParseIntError {
            full_version: "1.abc.3".to_owned(),
            version_part: "abc".to_owned(),
            parse_error: "invalid digit found in string".to_owned(),
        })
    );
    assert_eq!(
        parse("1.2.-3"),
        Err(VersionParseError::ParseIntError {
            full_version: "1.2.-3".to_owned(),
            version_part: "-3".to_owned(),
            parse_error: "invalid digit found in string".to_owned(),
        })
    );
    assert_eq!(
        parse("1.2.9876543210"),
        Err(VersionParseError::ParseIntError {
            full_version: "1.2.9876543210".to_owned(),
            version_part: "9876543210".to_owned(),
            parse_error: "number too large to fit in target type".to_owned(),
        })
    );
    assert_eq!(
        parse("1.2"),
        Err(VersionParseError::NotThreeParts {
            full_version: "1.2".to_owned(),
        })
    );
    assert_eq!(
        parse("1.2.3."),
        Err(VersionParseError::NotThreeParts {
            full_version: "1.2.3.".to_owned(),
        })
    );
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// Implement Version for SemanticVersion.
impl Version for SemanticVersion {
    fn minimum() -> Bound<Self> {
        Bound::Included(Self::zero())
    }
    fn maximum() -> Bound<Self> {
        Bound::Unbounded
    }
}

// SemanticInterval ############################################################

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SemanticInterval {
    start: Bound<SemanticVersion>,
    end: Bound<SemanticVersion>,
}

impl RangeBounds<SemanticVersion> for SemanticInterval {
    fn start_bound(&self) -> Bound<&SemanticVersion> {
        ref_bound(&self.start)
    }
    fn end_bound(&self) -> Bound<&SemanticVersion> {
        ref_bound(&self.end)
    }
}

impl Interval<SemanticVersion> for SemanticInterval {
    fn new(start_bound: Bound<SemanticVersion>, end_bound: Bound<SemanticVersion>) -> Self {
        let start = match &start_bound {
            Bound::Unbounded => Bound::Included(SemanticVersion::zero()),
            _ => start_bound,
        };
        Self {
            start,
            end: end_bound,
        }
    }
}

// Ease the creation of SemanticInterval from classic ranges: (2, 0, 0)..(2, 5, 0)
impl<RB: RangeBounds<(u32, u32, u32)>> From<RB> for SemanticInterval {
    fn from(range: RB) -> Self {
        Self::new(
            map_bound(SemanticVersion::from, owned_bound(range.start_bound())),
            map_bound(SemanticVersion::from, owned_bound(range.end_bound())),
        )
    }
}

// NumberVersion ###############################################################

/// Simplest versions possible, just a positive number.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize,))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct NumberVersion(pub u32);

// Convert an usize into a version.
impl From<u32> for NumberVersion {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

// Convert a version into an usize.
impl From<NumberVersion> for u32 {
    fn from(version: NumberVersion) -> Self {
        version.0
    }
}

impl Display for NumberVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Version for NumberVersion {
    fn minimum() -> Bound<Self> {
        Bound::Included(NumberVersion(0))
    }
    fn maximum() -> Bound<Self> {
        Bound::Unbounded
    }
}

impl NumberVersion {
    fn bump(&self) -> Self {
        NumberVersion(self.0 + 1)
    }
}

// NumberInterval ##############################################################

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NumberInterval {
    start: NumberVersion,
    end: Option<NumberVersion>,
}

// Ease the creation of NumberInterval from classic ranges: 0..10
impl<RB: RangeBounds<u32>> From<RB> for NumberInterval {
    fn from(range: RB) -> Self {
        Self::new(
            map_bound(NumberVersion::from, owned_bound(range.start_bound())),
            map_bound(NumberVersion::from, owned_bound(range.end_bound())),
        )
    }
}

impl RangeBounds<NumberVersion> for NumberInterval {
    fn start_bound(&self) -> Bound<&NumberVersion> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&NumberVersion> {
        match &self.end {
            None => Bound::Unbounded,
            Some(v) => Bound::Excluded(v),
        }
    }
}

impl Interval<NumberVersion> for NumberInterval {
    fn new(start_bound: Bound<NumberVersion>, end_bound: Bound<NumberVersion>) -> Self {
        let start = match start_bound {
            Bound::Unbounded => NumberVersion(0),
            Bound::Excluded(v) => v.bump(),
            Bound::Included(v) => v,
        };
        let end = match end_bound {
            Bound::Unbounded => None,
            Bound::Excluded(v) => Some(v),
            Bound::Included(v) => Some(v.bump()),
        };
        Self { start, end }
    }
}

// Helper ##################################################################

pub fn map_bound<V, T, F: Fn(V) -> T>(f: F, bound: Bound<V>) -> Bound<T> {
    match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Excluded(b) => Bound::Included(f(b)),
        Bound::Included(b) => Bound::Excluded(f(b)),
    }
}

pub fn flip_bound<V>(bound: Bound<V>) -> Bound<V> {
    match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Excluded(b) => Bound::Included(b),
        Bound::Included(b) => Bound::Excluded(b),
    }
}

/// Will be obsolete when .as_ref() hits stable.
pub fn ref_bound<V>(bound: &Bound<V>) -> Bound<&V> {
    match *bound {
        Bound::Included(ref x) => Bound::Included(x),
        Bound::Excluded(ref x) => Bound::Excluded(x),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// Will be obsolete when .cloned() hits stable.
pub fn owned_bound<V: Clone>(bound: Bound<&V>) -> Bound<V> {
    match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(x) => Bound::Included(x.clone()),
        Bound::Excluded(x) => Bound::Excluded(x.clone()),
    }
}
