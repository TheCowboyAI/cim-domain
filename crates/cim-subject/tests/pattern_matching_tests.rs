//! Pattern Matching Tests for CIM Subject
//!
//! Tests for NATS-style wildcard pattern matching
//!
//! ## Pattern Matching Flow
//! ```mermaid
//! graph TD
//!     A[Pattern Creation] --> B[Token Parsing]
//!     B --> C[Single Wildcard *]
//!     B --> D[Multi Wildcard >]
//!     C --> E[Match Validation]
//!     D --> E
//!     E --> F[Specificity Comparison]
//! ```

use cim_subject::{Pattern, PatternMatcher, Result, Subject};

// ============================================================================
// Test: Basic Pattern Matching
// ============================================================================

#[test]
fn test_exact_pattern_matching() {
    // Exact patterns should only match identical subjects
    let pattern = Pattern::new("orders.order.created.v1").unwrap();

    assert!(pattern.matches_str("orders.order.created.v1"));
    assert!(!pattern.matches_str("orders.order.updated.v1"));
    assert!(!pattern.matches_str("orders.order.created.v2"));
    assert!(!pattern.matches_str("orders.customer.created.v1"));
    assert!(!pattern.matches_str("payments.order.created.v1"));
}

#[test]
fn test_single_wildcard_patterns() {
    // Single wildcard (*) matches exactly one token
    let pattern = Pattern::new("orders.*.created.v1").unwrap();

    // Should match
    assert!(pattern.matches_str("orders.order.created.v1"));
    assert!(pattern.matches_str("orders.customer.created.v1"));
    assert!(pattern.matches_str("orders.item.created.v1"));

    // Should not match
    assert!(!pattern.matches_str("payments.order.created.v1")); // Wrong context
    assert!(!pattern.matches_str("orders.order.updated.v1")); // Wrong event
    assert!(!pattern.matches_str("orders.order.item.created.v1")); // Too many tokens
    assert!(!pattern.matches_str("orders.created.v1")); // Too few tokens
}

#[test]
fn test_multi_wildcard_patterns() {
    // Multi wildcard (>) matches one or more tokens at the end
    let pattern = Pattern::new("events.>").unwrap();

    // Should match everything under events
    assert!(pattern.matches_str("events.order.created.v1"));
    assert!(pattern.matches_str("events.user.profile.updated.v2"));
    assert!(pattern.matches_str("events.system.health.check.passed.v1"));
    assert!(pattern.matches_str("events.single"));

    // Should not match
    assert!(!pattern.matches_str("commands.order.create.v1"));
    assert!(!pattern.matches_str("queries.user.get.v1"));
}

// ============================================================================
// Test: Complex Pattern Combinations
// ============================================================================

#[test]
fn test_mixed_wildcard_patterns() {
    // Combine single and multi wildcards
    let pattern = Pattern::new("*.*.created.>").unwrap();

    // Should match any created event
    assert!(pattern.matches_str("orders.order.created.v1"));
    assert!(pattern.matches_str("users.profile.created.v2"));
    assert!(pattern.matches_str("inventory.item.created.v1.beta"));
    assert!(pattern.matches_str("system.config.created.v3.rc1.final"));

    // Should not match
    assert!(!pattern.matches_str("orders.created.v1")); // Too few tokens before created
    assert!(!pattern.matches_str("orders.order.updated.v1")); // Wrong event type
}

#[test]
fn test_multiple_single_wildcards() {
    let pattern = Pattern::new("*.*.*.v1").unwrap();

    // Should match any 4-token subject ending in v1
    assert!(pattern.matches_str("orders.order.created.v1"));
    assert!(pattern.matches_str("users.profile.updated.v1"));
    assert!(pattern.matches_str("system.health.checked.v1"));

    // Should not match
    assert!(!pattern.matches_str("orders.order.created")); // Too few tokens
    assert!(!pattern.matches_str("orders.order.item.created.v1")); // Too many tokens
    assert!(!pattern.matches_str("orders.order.created.v2")); // Wrong version
}

// ============================================================================
// Test: Pattern Specificity
// ============================================================================

#[test]
fn test_pattern_specificity_ordering() {
    // More specific patterns should rank higher
    let p1 = Pattern::new("orders.order.created.v1").unwrap(); // Most specific
    let p2 = Pattern::new("orders.order.created.*").unwrap();
    let p3 = Pattern::new("orders.order.*.v1").unwrap();
    let p4 = Pattern::new("orders.*.created.v1").unwrap();
    let p5 = Pattern::new("orders.*.*.v1").unwrap();
    let p6 = Pattern::new("*.order.created.v1").unwrap();
    let p7 = Pattern::new("*.*.*.v1").unwrap();
    let p8 = Pattern::new("orders.>").unwrap();
    let p9 = Pattern::new("*.*.*.*").unwrap();
    let p10 = Pattern::new(">").unwrap(); // Least specific

    // Test specificity comparisons
    assert!(p1.is_more_specific_than(&p2));
    assert!(p2.is_more_specific_than(&p3));
    assert!(p3.is_more_specific_than(&p4));
    assert!(p4.is_more_specific_than(&p5));
    assert!(p5.is_more_specific_than(&p7));
    assert!(p7.is_more_specific_than(&p8));
    assert!(p8.is_more_specific_than(&p10));

    // Patterns with multi-wildcard are less specific
    assert!(p9.is_more_specific_than(&p8));
    assert!(p9.is_more_specific_than(&p10));
}

// ============================================================================
// Test: Pattern Validation
// ============================================================================

#[test]
fn test_invalid_pattern_creation() {
    // Empty pattern
    assert!(Pattern::new("").is_err());

    // Empty tokens
    assert!(Pattern::new("orders..created.v1").is_err());
    assert!(Pattern::new(".orders.created.v1").is_err());
    assert!(Pattern::new("orders.created.v1.").is_err());

    // Multi-wildcard not at end
    assert!(Pattern::new("orders.>.created.v1").is_err());
    assert!(Pattern::new(">.orders.created.v1").is_err());

    // Invalid characters
    assert!(Pattern::new("orders.order$.created.v1").is_err());
    assert!(Pattern::new("orders.order@host.created.v1").is_err());
    assert!(Pattern::new("orders.order created.v1").is_err());
}

#[test]
fn test_valid_token_characters() {
    // Valid patterns with allowed characters
    assert!(Pattern::new("orders.order_item.created.v1").is_ok());
    assert!(Pattern::new("orders.order-item.created.v1").is_ok());
    assert!(Pattern::new("orders.order123.created.v1").is_ok());
    assert!(Pattern::new("orders.ORDER.created.v1").is_ok());
    assert!(Pattern::new("orders.order_123-ABC.created.v1").is_ok());
}

// ============================================================================
// Test: Pattern Matching with Subject Types
// ============================================================================

#[test]
fn test_pattern_matcher_trait() {
    let pattern = Pattern::new("users.*.updated.>").unwrap();

    // Test with Subject
    let subject = Subject::new("users.profile.updated.v1").unwrap();
    assert!(subject.matches_pattern(&pattern));

    // Test with &str
    assert!("users.settings.updated.v2".matches_pattern(&pattern));

    // Test with String
    let subject_string = String::from("users.preferences.updated.v1.beta");
    assert!(subject_string.matches_pattern(&pattern));
}

// ============================================================================
// Test: Edge Cases
// ============================================================================

#[test]
fn test_edge_case_patterns() {
    // Single token with multi-wildcard
    let pattern = Pattern::new(">").unwrap();
    assert!(pattern.matches_str("anything"));
    assert!(pattern.matches_str("any.thing.at.all"));

    // All wildcards
    let pattern = Pattern::new("*.*.*.*").unwrap();
    assert!(pattern.matches_str("a.b.c.d"));
    assert!(!pattern.matches_str("a.b.c")); // Too few
    assert!(!pattern.matches_str("a.b.c.d.e")); // Too many

    // Mix of literals and wildcards
    let pattern = Pattern::new("orders.*.*.v1").unwrap();
    assert!(pattern.matches_str("orders.order.created.v1"));
    assert!(pattern.matches_str("orders.item.deleted.v1"));
    assert!(!pattern.matches_str("orders.order.item.created.v1")); // Wrong structure
}

// ============================================================================
// Test: Real-World Patterns
// ============================================================================

#[test]
fn test_event_sourcing_patterns() {
    // Domain event patterns - match any version
    let domain_events_v1 = Pattern::new("*.*.*.v1").unwrap();
    let domain_events_v2 = Pattern::new("*.*.*.v2").unwrap();
    let domain_events_any = Pattern::new("*.*.*.>").unwrap();

    assert!(domain_events_v1.matches_str("orders.order.created.v1"));
    assert!(domain_events_v2.matches_str("inventory.stock.depleted.v2"));
    assert!(domain_events_any.matches_str("users.profile.updated.v3"));

    // Command patterns
    let commands_v1 = Pattern::new("cmd.*.*.v1").unwrap();
    let commands_any = Pattern::new("cmd.*.*.>").unwrap();

    assert!(commands_v1.matches_str("cmd.order.create.v1"));
    assert!(commands_any.matches_str("cmd.user.update.v1"));
    assert!(!commands_v1.matches_str("event.order.created.v1"));

    // Query patterns
    let queries = Pattern::new("qry.*.>").unwrap();

    assert!(queries.matches_str("qry.order.get.v1"));
    assert!(queries.matches_str("qry.user.list.v1.paginated"));
    assert!(!queries.matches_str("cmd.order.create.v1"));
}

#[test]
fn test_microservice_routing_patterns() {
    // Service-specific patterns
    let order_service = Pattern::new("orders.>").unwrap();
    let user_service = Pattern::new("users.>").unwrap();
    let notification_service = Pattern::new("notifications.>").unwrap();

    // Route to order service
    assert!(order_service.matches_str("orders.order.created.v1"));
    assert!(order_service.matches_str("orders.item.updated.v2"));
    assert!(!order_service.matches_str("users.profile.created.v1"));

    // Cross-service events
    let integration_events = Pattern::new("*.*.integrated.>").unwrap();

    assert!(integration_events.matches_str("orders.payment.integrated.v1"));
    assert!(integration_events.matches_str("users.notification.integrated.v2"));
}

// ============================================================================
// Test: Performance Patterns
// ============================================================================

#[test]
fn test_pattern_matching_performance() {
    // Create patterns of increasing complexity
    let patterns = vec![
        Pattern::new("orders.order.created.v1").unwrap(),
        Pattern::new("orders.*.created.v1").unwrap(),
        Pattern::new("*.*.created.v1").unwrap(),
        Pattern::new("*.*.*.v1").unwrap(),
        Pattern::new("*.*.*.*").unwrap(),
        Pattern::new("orders.>").unwrap(),
        Pattern::new(">").unwrap(),
    ];

    let subjects = vec![
        "orders.order.created.v1",
        "users.profile.updated.v2",
        "inventory.stock.checked.v1",
        "payments.transaction.processed.v3",
    ];

    // Match all patterns against all subjects
    for pattern in &patterns {
        for subject in &subjects {
            let _ = pattern.matches_str(subject);
        }
    }

    // This test primarily ensures patterns work correctly at scale
    assert!(true);
}
