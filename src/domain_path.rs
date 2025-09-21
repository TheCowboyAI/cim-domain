//! DomainPath algebra for hierarchical domain object addressing.
//!
//! Domain paths follow the canonical dotted hierarchy: `cim.domain.<bounded_context>
//! .<facet>.<name>...`. The first two segments (`cim` and `domain`) are fixed; the
//! remaining segments describe bounded contexts, artifact kinds (e.g. `command`,
//! `event`, `value`), and nested domain components. The algebra forms a free
//! monoid under concatenation with `cim.domain` as the identity element.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::formal_domain::{DomainConcept, ValueObject};

/// Errors produced when parsing or manipulating domain paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainPathError {
    /// Encountered an empty segment (e.g. `cim..domain`).
    EmptySegment,
    /// A segment contained unsupported characters.
    InvalidSegment(String),
    /// The path did not start with the required `cim.domain` prefix.
    InvalidPrefix,
}

impl fmt::Display for DomainPathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DomainPathError::EmptySegment => f.write_str("domain path segments cannot be empty"),
            DomainPathError::InvalidSegment(seg) => {
                write!(f, "invalid domain path segment: {seg}")
            }
            DomainPathError::InvalidPrefix => {
                f.write_str("domain path must start with 'cim.domain'")
            }
        }
    }
}

impl std::error::Error for DomainPathError {}

/// A single validated path segment (does not include dots).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomainPathSegment(String);

impl DomainPathSegment {
    /// Construct a segment after validating its contents.
    pub fn new(segment: impl Into<String>) -> Result<Self, DomainPathError> {
        let segment = segment.into();
        if segment.is_empty() {
            return Err(DomainPathError::EmptySegment);
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
        {
            return Err(DomainPathError::InvalidSegment(segment));
        }
        Ok(Self(segment))
    }

    /// View the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for DomainPathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Known domain artifact kinds used in canonical paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainArtifactKind {
    /// Aggregate facet (aggregate root / entity boundary).
    Aggregate,
    /// Command facet.
    Command,
    /// Event facet.
    Event,
    /// Policy facet.
    Policy,
    /// Projection facet.
    Projection,
    /// Query facet.
    Query,
    /// Read model facet.
    ReadModel,
    /// Saga facet.
    Saga,
    /// State machine facet.
    StateMachine,
    /// Value object facet.
    Value,
    /// Entity facet.
    Entity,
    /// Items facet (collection/value slice).
    Items,
}

impl DomainArtifactKind {
    /// Parse a facet segment into a known artifact kind.
    pub fn from_segment(seg: &str) -> Option<Self> {
        match seg {
            "aggregate" => Some(Self::Aggregate),
            "command" => Some(Self::Command),
            "event" => Some(Self::Event),
            "policy" => Some(Self::Policy),
            "projection" => Some(Self::Projection),
            "query" => Some(Self::Query),
            "read_model" => Some(Self::ReadModel),
            "saga" => Some(Self::Saga),
            "state_machine" => Some(Self::StateMachine),
            "value" => Some(Self::Value),
            "entity" => Some(Self::Entity),
            "items" => Some(Self::Items),
            _ => None,
        }
    }

    /// Return the canonical segment string for this artifact kind.
    pub fn segment(self) -> &'static str {
        match self {
            Self::Aggregate => "aggregate",
            Self::Command => "command",
            Self::Event => "event",
            Self::Policy => "policy",
            Self::Projection => "projection",
            Self::Query => "query",
            Self::ReadModel => "read_model",
            Self::Saga => "saga",
            Self::StateMachine => "state_machine",
            Self::Value => "value",
            Self::Entity => "entity",
            Self::Items => "items",
        }
    }
}

/// Canonical dotted domain path (e.g. `cim.domain.order.command.neworder`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomainPath {
    segments: Vec<DomainPathSegment>,
}

impl DomainPath {
    const ROOT_PREFIX: [&'static str; 2] = ["cim", "domain"];

    /// Identity element of the algebra (`cim.domain`).
    pub fn root() -> Self {
        let segments = Self::ROOT_PREFIX
            .iter()
            .map(|s| DomainPathSegment::new(*s).expect("static segment"))
            .collect();
        Self { segments }
    }

    /// Parse a path from a dotted string (enforces the `cim.domain` prefix).
    pub fn parse(input: &str) -> Result<Self, DomainPathError> {
        if input.is_empty() {
            return Ok(Self::root());
        }
        let mut segments = Vec::new();
        for raw in input.split('.') {
            if raw.is_empty() {
                return Err(DomainPathError::EmptySegment);
            }
            segments.push(DomainPathSegment::new(raw)?);
        }
        if segments.len() < 2
            || segments[0].as_str() != Self::ROOT_PREFIX[0]
            || segments[1].as_str() != Self::ROOT_PREFIX[1]
        {
            return Err(DomainPathError::InvalidPrefix);
        }
        Ok(Self { segments })
    }

    /// Return the number of segments.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// True when the path contains no segments.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// True when the path is exactly the root prefix.
    pub fn is_root(&self) -> bool {
        self.segments.len() == Self::ROOT_PREFIX.len()
    }

    /// Iterate over segments.
    pub fn segments(&self) -> impl Iterator<Item = &DomainPathSegment> {
        self.segments.iter()
    }

    /// Append a new segment, returning a new path.
    pub fn append(&self, segment: DomainPathSegment) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment);
        Self { segments }
    }

    /// Concatenate with another path, reusing the algebra identity.
    pub fn concat(&self, other: &DomainPath) -> Self {
        if self.is_root() {
            return other.clone();
        }
        if other.is_root() {
            return self.clone();
        }
        let mut segments = self.segments.clone();
        segments.extend(other.segments.iter().skip(Self::ROOT_PREFIX.len()).cloned());
        Self { segments }
    }

    /// Bounded context (3rd segment) if present.
    pub fn bounded_context(&self) -> Option<&str> {
        self.segments.get(2).map(|seg| seg.as_str())
    }

    /// Canonical artifact kind (4th segment) where recognised.
    pub fn artifact_kind(&self) -> Option<DomainArtifactKind> {
        self.segments
            .get(3)
            .and_then(|seg| DomainArtifactKind::from_segment(seg.as_str()))
    }

    /// Primary artifact name (5th segment) when available.
    pub fn artifact_name(&self) -> Option<&str> {
        self.segments.get(4).map(|seg| seg.as_str())
    }

    /// Create a path for an aggregate in the provided bounded context.
    pub fn aggregate(bounded_context: &str, name: &str) -> Result<Self, DomainPathError> {
        Self::root()
            .with_context(bounded_context)?
            .with_facet(DomainArtifactKind::Aggregate.segment())?
            .with_name(name)
    }

    /// Create a path for a command within a bounded context.
    pub fn command(bounded_context: &str, name: &str) -> Result<Self, DomainPathError> {
        Self::root()
            .with_context(bounded_context)?
            .with_facet(DomainArtifactKind::Command.segment())?
            .with_name(name)
    }

    /// Create a path for a value object/property within a bounded context.
    pub fn value(bounded_context: &str, scope: &str, name: &str) -> Result<Self, DomainPathError> {
        let mut path = Self::root().with_context(bounded_context)?;
        path = path.with_facet(scope)?;
        path.with_name(name)
    }

    fn with_context(mut self, context: &str) -> Result<Self, DomainPathError> {
        let ctx = DomainPathSegment::new(context)?;
        if self.len() > Self::ROOT_PREFIX.len() {
            return Err(DomainPathError::InvalidPrefix);
        }
        self.segments.push(ctx);
        Ok(self)
    }

    fn with_facet(mut self, facet: &str) -> Result<Self, DomainPathError> {
        if self.len() < Self::ROOT_PREFIX.len() + 1 {
            return Err(DomainPathError::InvalidPrefix);
        }
        self.segments.push(DomainPathSegment::new(facet)?);
        Ok(self)
    }

    fn with_name(mut self, name: &str) -> Result<Self, DomainPathError> {
        self.segments.push(DomainPathSegment::new(name)?);
        Ok(self)
    }
}

impl Display for DomainPath {
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

impl FromStr for DomainPath {
    type Err = DomainPathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DomainPath::parse(s)
    }
}

impl DomainConcept for DomainPath {}
impl ValueObject for DomainPath {}
impl DomainConcept for DomainPathSegment {}
impl ValueObject for DomainPathSegment {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_path_roundtrip() {
        let input = "cim.domain.person.policy.enroll";
        let path = DomainPath::parse(input).unwrap();
        assert_eq!(path.to_string(), input);
        assert_eq!(path.bounded_context(), Some("person"));
        assert_eq!(path.artifact_kind(), Some(DomainArtifactKind::Policy));
        assert_eq!(path.artifact_name(), Some("enroll"));
    }

    #[test]
    fn domain_path_concat_obeys_identity() {
        let root = DomainPath::root();
        let person = DomainPath::parse("cim.domain.person").unwrap();
        let policy = DomainPath::parse("cim.domain.person.policy.enroll").unwrap();
        assert_eq!(root.concat(&policy), policy);
        assert_eq!(person.concat(&root), person);
    }

    #[test]
    fn domain_path_concat_associative() {
        let a = DomainPath::parse("cim.domain.person").unwrap();
        let b = DomainPath::parse("cim.domain.person.policy").unwrap();
        let c = DomainPath::parse("cim.domain.person.policy.enroll").unwrap();
        assert_eq!(a.concat(&b).concat(&c), a.concat(&b.concat(&c)));
    }

    #[test]
    fn builders_enforce_prefix() {
        let command = DomainPath::command("person", "register").unwrap();
        assert_eq!(command.to_string(), "cim.domain.person.command.register");
        let value = DomainPath::value("organization", "location", "primary").unwrap();
        assert_eq!(
            value.to_string(),
            "cim.domain.organization.location.primary"
        );
    }
}
