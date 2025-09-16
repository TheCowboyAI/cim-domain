use cim_domain::DomainEvent;

pub fn event_types(events: &[Box<dyn DomainEvent>]) -> Vec<String> {
    events.iter().map(|e| e.event_type().to_string()).collect()
}

pub fn assert_event_types(events: &[Box<dyn DomainEvent>], expected: &[&str]) {
    let got = event_types(events);
    let exp: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, exp, "event types mismatch: got={:?} expected={:?}", got, exp);
}

pub trait ExpectEvents {
    fn expect_types(&self, expected: &[&str]);
}

impl ExpectEvents for Vec<Box<dyn DomainEvent>> {
    fn expect_types(&self, expected: &[&str]) { assert_event_types(self, expected) }
}

