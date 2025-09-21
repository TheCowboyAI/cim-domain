// Copyright (c) 2025 - Cowboy AI, LLC.

use cim_domain::{DomainArtifactKind, DomainPath, DomainPathError};

#[test]
fn parse_roundtrip_and_accessors() {
    let path = DomainPath::parse("cim.domain.person.policy.enroll").unwrap();
    assert_eq!(path.to_string(), "cim.domain.person.policy.enroll");
    assert_eq!(path.bounded_context(), Some("person"));
    assert!(path.artifact_kind().is_some());
    assert_eq!(path.artifact_name(), Some("enroll"));
}

#[test]
fn concat_identity_and_associativity() {
    let root = DomainPath::root();
    let person = DomainPath::parse("cim.domain.person").unwrap();
    let policy = DomainPath::parse("cim.domain.person.policy").unwrap();
    let enroll = DomainPath::parse("cim.domain.person.policy.enroll").unwrap();
    assert_eq!(root.concat(&person), person);
    assert_eq!(enroll.concat(&root), enroll);

    let left = person.concat(&policy).concat(&enroll);
    let right = person.concat(&policy.concat(&enroll));
    assert_eq!(left, right);
}

#[test]
fn builder_helpers_cover_samples() {
    let command = DomainPath::command("person", "register").unwrap();
    assert_eq!(command.to_string(), "cim.domain.person.command.register");

    let value = DomainPath::value("organization", "location", "primary").unwrap();
    assert_eq!(
        value.to_string(),
        "cim.domain.organization.location.primary"
    );
}

#[test]
fn invalid_prefix_rejected() {
    let err = DomainPath::parse("domain.order.command").unwrap_err();
    assert!(matches!(err, DomainPathError::InvalidPrefix));
}

#[test]
fn artifact_kind_segments_round_trip() {
    let variants = [
        DomainArtifactKind::Aggregate,
        DomainArtifactKind::Command,
        DomainArtifactKind::Event,
        DomainArtifactKind::Policy,
        DomainArtifactKind::Projection,
        DomainArtifactKind::Query,
        DomainArtifactKind::ReadModel,
        DomainArtifactKind::Saga,
        DomainArtifactKind::StateMachine,
        DomainArtifactKind::Value,
        DomainArtifactKind::Entity,
        DomainArtifactKind::Items,
    ];

    for kind in variants {
        let segment = kind.segment();
        assert_eq!(DomainArtifactKind::from_segment(segment), Some(kind));
    }
}
