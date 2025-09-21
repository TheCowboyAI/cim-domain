// Copyright (c) 2025 - Cowboy AI, LLC.

use cim_domain::subject::{Subject, SubjectError, SubjectPattern, SubjectSegment};

#[test]
fn subject_parse_and_display_roundtrip() {
    let subject = Subject::parse("person.policy.issued").unwrap();
    assert_eq!(subject.to_string(), "person.policy.issued");
    assert_eq!(subject.len(), 3);
}

#[test]
fn subject_segment_validation_rejects_wildcards() {
    let err = SubjectSegment::new("*").unwrap_err();
    assert!(matches!(err, SubjectError::InvalidSegment(_)));
}

#[test]
fn subject_concat_is_associative_with_root_identity() {
    let s1 = Subject::parse("person").unwrap();
    let s2 = Subject::parse("organization").unwrap();
    let s3 = Subject::parse("location.primary").unwrap();

    let left = s1.concat(&s2).concat(&s3);
    let right = s1.concat(&s2.concat(&s3));
    assert_eq!(left, right);

    assert_eq!(Subject::root().concat(&s1), s1);
    assert_eq!(s1.concat(&Subject::root()), s1);
}

#[test]
fn subject_pattern_matches_single_and_multi_wildcards() {
    let subject = Subject::parse("person.policy.issued").unwrap();

    let single = SubjectPattern::parse("person.*.issued").unwrap();
    let multi = SubjectPattern::parse("person.policy.>").unwrap();
    let mismatch = SubjectPattern::parse("organization.policy.>").unwrap();

    assert!(subject.matches(&single));
    assert!(subject.matches(&multi));
    assert!(!subject.matches(&mismatch));
}

#[test]
fn subject_pattern_requires_terminal_multi_wildcard() {
    let err = SubjectPattern::parse("finance.>.created").unwrap_err();
    assert!(matches!(err, SubjectError::TrailingWildcardRequired));
}

#[test]
fn empty_string_parses_to_root() {
    let root = Subject::parse("").unwrap();
    assert!(root.is_root());
    assert_eq!(root.len(), 0);
}
