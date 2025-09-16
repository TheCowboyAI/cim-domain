//! Shared BDD helpers for integration tests.
//! Keep this file pure and dependency‑free.

use cim_domain::DomainEvent;

/// Collect event type names from a slice of DomainEvent trait objects.
pub fn event_types(events: &[Box<dyn DomainEvent>]) -> Vec<String> {
    events.iter().map(|e| e.event_type().to_string()).collect()
}

/// Assert that the sequence of event type names equals the expected list.
pub fn assert_event_types(events: &[Box<dyn DomainEvent>], expected: &[&str]) {
    let got = event_types(events);
    let exp: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, exp, "event types mismatch: got={:?} expected={:?}", got, exp);
}

/// Convenience trait for asserting expected event types on a vector of events.
pub trait ExpectEvents {
    fn expect_types(&self, expected: &[&str]);
}

impl ExpectEvents for Vec<Box<dyn DomainEvent>> {
    fn expect_types(&self, expected: &[&str]) {
        assert_event_types(self, expected)
    }
}

