//! Translator Tests for CIM Subject
//!
//! Tests for bidirectional subject translation between different schemas
//!
//! ## Translation Flow
//! ```mermaid
//! graph TD
//!     A[Source Subject] --> B[Translation Rule]
//!     B --> C[Pattern Match]
//!     C --> D[Field Extraction]
//!     D --> E[Target Construction]
//!     E --> F[Target Subject]
//!     F --> G[Reverse Translation]
//! ```

use cim_subject::{
    Pattern, Result, Subject, SubjectParts,
    translator::{TranslationRule, Translator, TranslatorBuilder},
};
use std::sync::Arc;

// ============================================================================
// Test: Basic Translation Rules
// ============================================================================

#[test]
fn test_simple_translation() {
    // Use TranslatorBuilder for simple mappings
    let translator = TranslatorBuilder::new()
        .map("internal.*.*.v1", "public.{aggregate}.{event}.v1")
        .unwrap()
        .build();

    // Translate internal subject to public
    let internal = Subject::new("internal.user.created.v1").unwrap();
    let public = translator.translate(&internal).unwrap();

    assert_eq!(public.as_str(), "public.user.created.v1");
}

#[test]
fn test_custom_translation_rule() {
    let translator = Translator::new();

    // Create a custom translation rule
    let rule = TranslationRule::new(
        "internal_to_public",
        Pattern::new("internal.*.*.v1").unwrap(),
        Arc::new(|subject| {
            let parts = SubjectParts::new(
                "public",
                subject.aggregate(),
                subject.event_type(),
                subject.version(),
            );
            Ok(Subject::from_parts(parts))
        }),
    );

    translator.register_rule("internal_to_public", rule);

    // Translate internal subject to public
    let internal = Subject::new("internal.user.created.v1").unwrap();
    let public = translator.translate(&internal).unwrap();

    assert_eq!(public.as_str(), "public.user.created.v1");
}

#[test]
fn test_context_translation() {
    // Use TranslatorBuilder for context translation
    let translator = TranslatorBuilder::new()
        .translate_context("dev", "prod")
        .unwrap()
        .translate_context("staging", "prod")
        .unwrap()
        .build();

    let dev_subject = Subject::new("dev.service.deployed.v1").unwrap();
    let prod_subject = translator.translate(&dev_subject).unwrap();

    assert_eq!(prod_subject.context(), "prod");
    assert_eq!(prod_subject.aggregate(), "service");
    assert_eq!(prod_subject.event_type(), "deployed");
}

// ============================================================================
// Test: Pattern-Based Translation
// ============================================================================

#[test]
fn test_wildcard_translation() {
    let translator = Translator::new();

    // Create custom rule for legacy to modern migration
    let rule = TranslationRule::new(
        "domain_migration",
        Pattern::new("legacy.>").unwrap(),
        Arc::new(|subject| {
            // Extract parts after "legacy."
            let parts: Vec<&str> = subject.as_str().split('.').collect();
            if parts.len() >= 3 {
                let parts = SubjectParts::new("modern", parts[1], parts[2], "v2");
                Ok(Subject::from_parts(parts))
            } else {
                Ok(subject.clone())
            }
        }),
    );

    translator.register_rule("domain_migration", rule);

    // Test various subjects
    let subjects = vec![
        ("legacy.orders.created.v1", "modern.orders.created.v2"),
        ("legacy.users.updated.v1", "modern.users.updated.v2"),
        (
            "legacy.inventory.depleted.v3",
            "modern.inventory.depleted.v2",
        ),
    ];

    for (source, expected) in subjects {
        let subject = Subject::new(source).unwrap();
        let translated = translator.translate(&subject).unwrap();
        assert_eq!(translated.as_str(), expected);
    }
}

// ============================================================================
// Test: Bidirectional Translation
// ============================================================================

#[test]
fn test_bidirectional_translation() {
    // Create a bidirectional translation rule
    let forward_rule = TranslationRule::new(
        "internal_external",
        Pattern::new("internal.>").unwrap(),
        Arc::new(|subject| {
            let parts = SubjectParts::new(
                "external",
                subject.aggregate(),
                subject.event_type(),
                subject.version(),
            );
            Ok(Subject::from_parts(parts))
        }),
    )
    .with_target_pattern(Pattern::new("external.>").unwrap())
    .with_reverse(Arc::new(|subject| {
        let parts = SubjectParts::new(
            "internal",
            subject.aggregate(),
            subject.event_type(),
            subject.version(),
        );
        Ok(Subject::from_parts(parts))
    }));

    let translator = Translator::bidirectional(vec![forward_rule], vec![]);

    // Test forward translation
    let internal = Subject::new("internal.service.started.v1").unwrap();
    let external = translator.translate(&internal).unwrap();
    assert_eq!(external.context(), "external");

    // Test reverse translation
    let back = translator.reverse_translate(&external).unwrap();
    assert_eq!(back.as_str(), internal.as_str());
}

// ============================================================================
// Test: Complex Translation Scenarios
// ============================================================================

#[test]
fn test_multi_stage_translation() {
    let translator = Translator::new();

    // Add multiple translation rules that can chain
    let step1 = TranslationRule::new(
        "v1_to_v2",
        Pattern::new("v1.*.*.event").unwrap(),
        Arc::new(|subject| {
            let parts: Vec<&str> = subject.as_str().split('.').collect();
            if parts.len() >= 4 {
                let new_subject = format!("v2.{parts[1]}.{parts[2]}.event");
                Subject::new(new_subject)
            } else {
                Ok(subject.clone())
            }
        }),
    );

    let step2 = TranslationRule::new(
        "v2_to_v3",
        Pattern::new("v2.*.*.event").unwrap(),
        Arc::new(|subject| {
            let parts: Vec<&str> = subject.as_str().split('.').collect();
            if parts.len() >= 4 {
                let new_subject = format!("v3.domain.{parts[1]}.{parts[2]}");
                Subject::new(new_subject)
            } else {
                Ok(subject.clone())
            }
        }),
    );

    translator.register_rule("step1", step1);
    translator.register_rule("step2", step2);

    // Translate through multiple stages
    let v1 = Subject::new("v1.orders.created.event").unwrap();
    let v2 = translator.translate(&v1).unwrap();
    assert_eq!(v2.as_str(), "v2.orders.created.event");

    let v3 = translator.translate(&v2).unwrap();
    assert_eq!(v3.as_str(), "v3.domain.orders.created");
}

#[test]
fn test_context_aware_translation() {
    let translator = Translator::new();

    // Different rules for different contexts
    let orders_rule = TranslationRule::new(
        "orders_context",
        Pattern::new("internal.orders.*.v1").unwrap(),
        Arc::new(|subject| {
            let parts = SubjectParts::new(
                "public",
                "commerce",
                subject.event_type(),
                subject.version(),
            );
            Ok(Subject::from_parts(parts))
        }),
    );

    let users_rule = TranslationRule::new(
        "users_context",
        Pattern::new("internal.users.*.v1").unwrap(),
        Arc::new(|subject| {
            let parts = SubjectParts::new(
                "public",
                "identity",
                subject.event_type(),
                subject.version(),
            );
            Ok(Subject::from_parts(parts))
        }),
    );

    translator.register_rule("orders", orders_rule);
    translator.register_rule("users", users_rule);

    // Test context-specific translations
    let order = Subject::new("internal.orders.created.v1").unwrap();
    let user = Subject::new("internal.users.registered.v1").unwrap();

    let public_order = translator.translate(&order).unwrap();
    let public_user = translator.translate(&user).unwrap();

    assert_eq!(public_order.as_str(), "public.commerce.created.v1");
    assert_eq!(public_user.as_str(), "public.identity.registered.v1");
}

// ============================================================================
// Test: Error Handling
// ============================================================================

#[test]
fn test_no_matching_rule() {
    let translator = Translator::new();

    // Try to translate without any rules
    let subject = Subject::new("unknown.subject.pattern.v1").unwrap();
    let result = translator.translate(&subject);

    // Without rules, it should return the original subject
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), subject.as_str());
}

// ============================================================================
// Test: Real-World Translation Scenarios
// ============================================================================

#[test]
fn test_legacy_system_migration() {
    let translator = TranslatorBuilder::new()
        .custom(
            "monolith_migration",
            TranslationRule::new(
                "monolith_to_services",
                Pattern::new("monolith.*.*.v1").unwrap(),
                Arc::new(|subject| {
                    let parts: Vec<&str> = subject.as_str().split('.').collect();
                    if parts.len() >= 4 {
                        let event_name = parts[2];

                        if event_name.starts_with("order_") {
                            let event_type = &event_name[6..]; // Remove "order_" prefix
                            let new_subject = format!("orders.order.{event_type}.v2");
                            Subject::new(new_subject)
                        } else if event_name.starts_with("inventory_") {
                            let event_type = &event_name[10..]; // Remove "inventory_" prefix
                            let new_subject = format!("inventory.stock.{event_type}.v2");
                            Subject::new(new_subject)
                        } else if event_name.starts_with("user_") {
                            let event_type = &event_name[5..]; // Remove "user_" prefix
                            let new_subject = format!("users.profile.{event_type}.v2");
                            Subject::new(new_subject)
                        } else {
                            Ok(subject.clone())
                        }
                    } else {
                        Ok(subject.clone())
                    }
                }),
            ),
        )
        .build();

    // Test various legacy events (need 4 parts for subject format)
    let migrations = vec![
        (
            "monolith.service.order_created.v1",
            "orders.order.created.v2",
        ),
        (
            "monolith.service.inventory_updated.v1",
            "inventory.stock.updated.v2",
        ),
        (
            "monolith.service.user_registered.v1",
            "users.profile.registered.v2",
        ),
    ];

    for (legacy, modern) in migrations {
        let subject = Subject::new(legacy).unwrap();
        let translated = translator.translate(&subject).unwrap();
        assert_eq!(translated.as_str(), modern);
    }
}
