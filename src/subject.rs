// Copyright (c) 2025 - Cowboy AI, LLC.

//! Subject algebra (value object) for composable domain addressing.
//!
//! Subjects form a free monoid over validated segments with concatenation as the
//! operation and the empty subject as identity. Patterns introduce the classic
//! single (`*`) and multi (`>`) segment wildcards used by subject-based routing
//! systems (e.g., NATS). All operations are pure and allocation is explicit.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::formal_domain::{DomainConcept, ValueObject};

/// Errors produced by subject parsing or manipulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectError {
    /// Found an empty segment between delimiters (e.g., `foo..bar`).
    EmptySegment,
    /// Segment contained disallowed characters.
    InvalidSegment(String),
    /// Wildcard `>` appeared in a non-terminal position.
    TrailingWildcardRequired,
}

impl fmt::Display for SubjectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SubjectError::EmptySegment => f.write_str("subject segments cannot be empty"),
            SubjectError::InvalidSegment(seg) => write!(f, "invalid subject segment: {seg}"),
            SubjectError::TrailingWildcardRequired => {
                f.write_str("multi-segment wildcard '>' must be the final segment")
            }
        }
    }
}

impl std::error::Error for SubjectError {}

/// A single validated subject segment (no wildcards or separators).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubjectSegment(String);

impl SubjectSegment {
    /// Create a new subject segment after validating allowed characters.
    pub fn new(segment: impl Into<String>) -> Result<Self, SubjectError> {
        let segment = segment.into();
        if segment.is_empty() {
            return Err(SubjectError::EmptySegment);
        }
        if !segment.chars().all(|ch| {
            !ch.is_control() && ch != '.' && ch != '*' && ch != '>' && !ch.is_whitespace()
        }) {
            return Err(SubjectError::InvalidSegment(segment));
        }
        Ok(Self(segment))
    }

    /// Access the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SubjectSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A concrete subject composed of validated segments separated by `.`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Subject {
    segments: Vec<SubjectSegment>,
}

impl Subject {
    /// Create the identity element (no segments).
    pub fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Construct a subject from the provided iterator of segments.
    pub fn from_segments<I>(segments: I) -> Result<Self, SubjectError>
    where
        I: IntoIterator<Item = SubjectSegment>,
    {
        let segments: Vec<_> = segments.into_iter().collect();
        Ok(Self { segments })
    }

    /// Parse a subject from a `.` delimited string. Empty input yields `root`.
    pub fn parse(input: &str) -> Result<Self, SubjectError> {
        if input.is_empty() {
            return Ok(Self::root());
        }
        let mut segments = Vec::new();
        for raw in input.split('.') {
            if raw.is_empty() {
                return Err(SubjectError::EmptySegment);
            }
            segments.push(SubjectSegment::new(raw)?);
        }
        Ok(Self { segments })
    }

    /// True when the subject has no segments.
    pub fn is_root(&self) -> bool {
        self.is_empty()
    }

    /// Alias required by Clippy: subjects are empty when they contain no segments.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Number of segments in the subject.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Iterate over the segments.
    pub fn segments(&self) -> impl Iterator<Item = &SubjectSegment> {
        self.segments.iter()
    }

    /// Append a segment, returning a new subject.
    pub fn append(&self, segment: SubjectSegment) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment);
        Self { segments }
    }

    /// Concatenate two subjects (monoid operation).
    pub fn concat(&self, other: &Subject) -> Self {
        if self.is_root() {
            return other.clone();
        }
        if other.is_root() {
            return self.clone();
        }
        let mut segments = self.segments.clone();
        segments.extend(other.segments.iter().cloned());
        Self { segments }
    }

    /// Determine whether this subject matches the provided pattern.
    pub fn matches(&self, pattern: &SubjectPattern) -> bool {
        pattern.matches(self)
    }
}

impl Display for Subject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rendered = self
            .segments
            .iter()
            .map(|seg| seg.as_str())
            .collect::<Vec<_>>()
            .join(".");
        f.write_str(&rendered)
    }
}

impl FromStr for Subject {
    type Err = SubjectError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Subject::parse(s)
    }
}

impl DomainConcept for Subject {}
impl ValueObject for Subject {}
impl DomainConcept for SubjectSegment {}
impl ValueObject for SubjectSegment {}

/// Pattern segment allowing literal, single (`*`), or multi (`>`) matches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubjectPatternSegment {
    /// Match an exact literal segment.
    Literal(SubjectSegment),
    /// Match any single segment.
    SingleWildcard,
    /// Match zero or more trailing segments.
    MultiWildcard,
}

/// Subject pattern with wildcard support.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubjectPattern {
    segments: Vec<SubjectPatternSegment>,
}

impl DomainConcept for SubjectPattern {}
impl ValueObject for SubjectPattern {}

impl DomainConcept for SubjectPatternSegment {}
impl ValueObject for SubjectPatternSegment {}

impl SubjectPattern {
    /// Parse a subject pattern from a `.` delimited string.
    pub fn parse(input: &str) -> Result<Self, SubjectError> {
        if input.is_empty() {
            return Ok(Self {
                segments: Vec::new(),
            });
        }
        let parts: Vec<&str> = input.split('.').collect();
        let mut segments = Vec::new();
        for (idx, raw) in parts.iter().enumerate() {
            if raw.is_empty() {
                return Err(SubjectError::EmptySegment);
            }
            let seg = match *raw {
                "*" => SubjectPatternSegment::SingleWildcard,
                ">" => {
                    if idx != parts.len() - 1 {
                        return Err(SubjectError::TrailingWildcardRequired);
                    }
                    SubjectPatternSegment::MultiWildcard
                }
                literal => SubjectPatternSegment::Literal(SubjectSegment::new(literal)?),
            };
            segments.push(seg);
        }
        Ok(Self { segments })
    }

    /// True when the pattern matches the subject.
    pub fn matches(&self, subject: &Subject) -> bool {
        self.matches_internal(subject, 0, 0)
    }

    fn matches_internal(&self, subject: &Subject, pattern_idx: usize, subject_idx: usize) -> bool {
        if pattern_idx == self.segments.len() {
            return subject_idx == subject.len();
        }

        match &self.segments[pattern_idx] {
            SubjectPatternSegment::Literal(expected) => {
                if subject_idx >= subject.len() {
                    return false;
                }
                if subject.segments[subject_idx] != *expected {
                    return false;
                }
                self.matches_internal(subject, pattern_idx + 1, subject_idx + 1)
            }
            SubjectPatternSegment::SingleWildcard => {
                if subject_idx >= subject.len() {
                    return false;
                }
                self.matches_internal(subject, pattern_idx + 1, subject_idx + 1)
            }
            SubjectPatternSegment::MultiWildcard => {
                // Since '>' must be terminal we can match remaining segments.
                true
            }
        }
    }
}

impl Display for SubjectPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rendered = self
            .segments
            .iter()
            .map(|segment| match segment {
                SubjectPatternSegment::Literal(lit) => lit.as_str(),
                SubjectPatternSegment::SingleWildcard => "*",
                SubjectPatternSegment::MultiWildcard => ">",
            })
            .collect::<Vec<_>>()
            .join(".");
        f.write_str(&rendered)
    }
}

impl FromStr for SubjectPattern {
    type Err = SubjectError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SubjectPattern::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_roundtrip() {
        let subject = Subject::parse("person.policy.issued").unwrap();
        assert_eq!(subject.to_string(), "person.policy.issued");
        assert_eq!(subject.len(), 3);
    }

    #[test]
    fn subject_concat_associative() {
        let a = Subject::parse("person").unwrap();
        let b = Subject::parse("organization").unwrap();
        let c = Subject::parse("location.primary").unwrap();

        let left = a.concat(&b).concat(&c);
        let right = a.concat(&b.concat(&c));
        assert_eq!(left, right);
    }

    #[test]
    fn pattern_matching_semantics() {
        let subject = Subject::parse("person.policy.issued").unwrap();
        let pattern_one = SubjectPattern::parse("person.*.issued").unwrap();
        let pattern_two = SubjectPattern::parse("person.policy.>").unwrap();
        let pattern_three = SubjectPattern::parse("organization.*.issued").unwrap();

        assert!(subject.matches(&pattern_one));
        assert!(subject.matches(&pattern_two));
        assert!(!subject.matches(&pattern_three));
    }

    #[test]
    fn root_behaves_as_identity() {
        let root = Subject::root();
        let person = Subject::parse("person.identity.created").unwrap();
        assert_eq!(root.concat(&person), person);
        assert_eq!(person.concat(&root), person);
        assert!(root.is_root());
    }

    #[test]
    fn pattern_requires_terminal_multi_wildcard() {
        let err = SubjectPattern::parse("orders.>.created").unwrap_err();
        assert_eq!(err, SubjectError::TrailingWildcardRequired);
    }
}
