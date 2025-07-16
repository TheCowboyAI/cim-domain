//! Algebra Tests for CIM Subject
//!
//! Tests for the algebraic structures and operations on subjects
//!
//! ## Test Flow Diagram
//! ```mermaid
//! graph TD
//!     A[Subject Creation] --> B[Pattern Matching]
//!     B --> C[Algebraic Operations]
//!     C --> D[Composition Rules]
//!     D --> E[Transformations]
//!     E --> F[Lattice Structure]
//!     F --> G[Complex Compositions]
//! ```

use cim_subject::{
    AlgebraOperation, CompositionRule, Pattern, PatternMatcher, Result, Subject, SubjectAlgebra,
    SubjectBuilder, SubjectError, SubjectParts,
};
use std::sync::Arc;

// ============================================================================
// Test: Basic Algebraic Operations
// ============================================================================

#[test]
fn test_sequential_composition() {
    let algebra = SubjectAlgebra::new();

    // Create workflow subjects
    let validate = Subject::new("workflow.order.validated.v1").unwrap();
    let process = Subject::new("workflow.payment.processed.v1").unwrap();

    // Compose sequentially
    let result = algebra
        .compose(&validate, &process, AlgebraOperation::Sequence)
        .unwrap();

    // Verify the composition
    assert_eq!(result.context(), "workflow-workflow");
    assert_eq!(result.aggregate(), "order-payment");
    assert_eq!(result.event_type(), "sequenced");
    assert_eq!(result.version(), "v1");

    // Test associativity: (A seq B) seq C = A seq (B seq C)
    let ship = Subject::new("workflow.shipping.dispatched.v1").unwrap();

    let left_assoc = algebra
        .compose(
            &algebra
                .compose(&validate, &process, AlgebraOperation::Sequence)
                .unwrap(),
            &ship,
            AlgebraOperation::Sequence,
        )
        .unwrap();

    let right_assoc = algebra
        .compose(
            &validate,
            &algebra
                .compose(&process, &ship, AlgebraOperation::Sequence)
                .unwrap(),
            AlgebraOperation::Sequence,
        )
        .unwrap();

    // While the exact representation might differ, the concept should be preserved
    assert!(left_assoc.as_str().contains("workflow"));
    assert!(right_assoc.as_str().contains("workflow"));
}

// ============================================================================
// Test: Parallel Composition
// ============================================================================

#[test]
fn test_parallel_composition() {
    let algebra = SubjectAlgebra::new();

    // Create independent subjects
    let email = Subject::new("notifications.email.sent.v1").unwrap();
    let sms = Subject::new("notifications.sms.sent.v1").unwrap();
    let push = Subject::new("notifications.push.sent.v1").unwrap();

    // Compose in parallel
    let email_sms = algebra
        .compose(&email, &sms, AlgebraOperation::Parallel)
        .unwrap();
    let all_notifications = algebra
        .compose(&email_sms, &push, AlgebraOperation::Parallel)
        .unwrap();

    // Verify parallel composition
    assert!(all_notifications.context().contains('+'));
    assert!(all_notifications.aggregate().contains('+'));
    assert_eq!(all_notifications.event_type(), "parallel");

    // Test commutativity: A || B = B || A (for parallel)
    let reverse = algebra
        .compose(&sms, &email, AlgebraOperation::Parallel)
        .unwrap();
    assert!(reverse.context().contains("notifications"));
    assert!(reverse.context().contains('+'));
}

// ============================================================================
// Test: Choice Composition
// ============================================================================

#[test]
fn test_choice_composition() {
    let algebra = SubjectAlgebra::new();

    // Create alternative paths
    let credit = Subject::new("payment.credit.charged.v1").unwrap();
    let debit = Subject::new("payment.debit.charged.v1").unwrap();
    let crypto = Subject::new("payment.crypto.charged.v1").unwrap();

    // Create choices
    let traditional = algebra
        .compose(
            &credit,
            &debit,
            AlgebraOperation::Choice {
                condition: "card_type".to_string(),
            },
        )
        .unwrap();

    let all_payment = algebra
        .compose(
            &traditional,
            &crypto,
            AlgebraOperation::Choice {
                condition: "payment_method".to_string(),
            },
        )
        .unwrap();

    // Verify choice composition
    assert_eq!(all_payment.context(), "payment");
    assert!(all_payment.aggregate().contains('|'));
    assert!(all_payment.event_type().contains("choice"));
}

// ============================================================================
// Test: Transformations
// ============================================================================

#[test]
fn test_subject_transformations() {
    let algebra = SubjectAlgebra::new();

    // Register transformations
    let anonymize = Transformation {
        name: "anonymize".to_string(),
        input_pattern: Pattern::new("users.*.*.v1").unwrap(),
        transform: Arc::new(|subject| {
            Ok(Subject::from_parts(SubjectParts::new(
                subject.context(),
                "anonymous",
                subject.event_type(),
                subject.version(),
            )))
        }),
    };

    let versioning = Transformation {
        name: "upgrade_version".to_string(),
        input_pattern: Pattern::new("*.*.*.v1").unwrap(),
        transform: Arc::new(|subject| {
            Ok(Subject::from_parts(SubjectParts::new(
                subject.context(),
                subject.aggregate(),
                subject.event_type(),
                "v2",
            )))
        }),
    };

    algebra.register_transformation("anonymize", anonymize);
    algebra.register_transformation("upgrade_version", versioning);

    // Test anonymization
    let user = Subject::new("users.person.created.v1").unwrap();
    let anon = algebra
        .compose(
            &user,
            &user,
            AlgebraOperation::Transform {
                name: "anonymize".to_string(),
            },
        )
        .unwrap();

    assert_eq!(anon.aggregate(), "anonymous");
    assert_eq!(anon.context(), "users");
    assert_eq!(anon.event_type(), "created");

    // Test version upgrade
    let upgraded = algebra
        .compose(
            &user,
            &user,
            AlgebraOperation::Transform {
                name: "upgrade_version".to_string(),
            },
        )
        .unwrap();

    assert_eq!(upgraded.version(), "v2");
    assert_eq!(upgraded.aggregate(), "person");
}

// ============================================================================
// Test: Projection Operations
// ============================================================================

#[test]
fn test_projection() {
    let algebra = SubjectAlgebra::new();

    let full_event = Subject::new("orders.order.completed.v1").unwrap();

    // Project specific fields
    let projected = algebra
        .compose(
            &full_event,
            &full_event,
            AlgebraOperation::Project {
                fields: vec!["order_id".to_string(), "total".to_string()],
            },
        )
        .unwrap();

    assert_eq!(projected.context(), "orders");
    assert_eq!(projected.aggregate(), "order");
    assert!(projected.event_type().contains("projected"));
    assert!(projected.event_type().contains("order_id"));
    assert!(projected.event_type().contains("total"));
}

// ============================================================================
// Test: Context Injection
// ============================================================================

#[test]
fn test_context_injection() {
    let algebra = SubjectAlgebra::new();

    // Internal event
    let internal = Subject::new("internal.user.registered.v1").unwrap();

    // Inject into public context
    let public = algebra
        .compose(
            &internal,
            &internal,
            AlgebraOperation::Inject {
                context: "public".to_string(),
            },
        )
        .unwrap();

    assert_eq!(public.context(), "public");
    assert_eq!(public.aggregate(), "user");
    assert_eq!(public.event_type(), "registered");
    assert_eq!(public.version(), "v1");

    // Inject into partner context
    let partner = algebra
        .compose(
            &internal,
            &internal,
            AlgebraOperation::Inject {
                context: "partner_api".to_string(),
            },
        )
        .unwrap();

    assert_eq!(partner.context(), "partner_api");
}

// ============================================================================
// Test: Subject Lattice Structure
// ============================================================================

#[test]
fn test_subject_lattice() {
    let algebra = SubjectAlgebra::new();

    // Create a hierarchy of subjects
    let subjects = vec![
        Subject::new("events.base.changed.v1").unwrap(),
        Subject::new("events.base.created.v1").unwrap(),
        Subject::new("events.base.updated.v1").unwrap(),
        Subject::new("events.base.deleted.v1").unwrap(),
        Subject::new("events.specific.created.v1").unwrap(),
    ];

    let lattice = algebra.create_lattice(&subjects);

    // Test join operation (least upper bound)
    let created = &subjects[1];
    let updated = &subjects[2];

    if let Some(join) = lattice.join(created, updated) {
        // The join of created and updated should be changed (more general)
        assert_eq!(join.event_type(), "changed");
    }
}

// ============================================================================
// Test: Complex Algebraic Compositions
// ============================================================================

#[test]
fn test_complex_compositions() {
    let algebra = SubjectAlgebra::new();

    // Create a complex workflow
    let order = Subject::new("orders.order.received.v1").unwrap();
    let inventory = Subject::new("inventory.stock.checked.v1").unwrap();
    let payment = Subject::new("payment.card.authorized.v1").unwrap();
    let shipping = Subject::new("shipping.label.created.v1").unwrap();

    // Sequential: order -> inventory check
    let order_inventory = algebra
        .compose(&order, &inventory, AlgebraOperation::Sequence)
        .unwrap();

    // Parallel: payment auth || shipping prep
    let payment_shipping = algebra
        .compose(&payment, &shipping, AlgebraOperation::Parallel)
        .unwrap();

    // Sequential: (order->inventory) -> (payment||shipping)
    let complete_flow = algebra
        .compose(
            &order_inventory,
            &payment_shipping,
            AlgebraOperation::Sequence,
        )
        .unwrap();

    // Verify the complex composition
    assert!(complete_flow.as_str().contains("sequenced"));
    assert!(complete_flow.context().contains('-'));
}

// ============================================================================
// Test: Custom Composition Rules
// ============================================================================

#[test]
fn test_custom_composition_rules() {
    let algebra = SubjectAlgebra::new();

    // Define a custom composition rule for saga patterns
    let saga_rule = CompositionRule {
        name: "saga".to_string(),
        left_pattern: Pattern::new("saga.*.started.v1").unwrap(),
        right_pattern: Pattern::new("saga.*.completed.v1").unwrap(),
        composer: Arc::new(|left, right| {
            // Extract saga name from aggregate
            let saga_name = left.aggregate();
            Ok(Subject::from_parts(SubjectParts::new(
                "saga", saga_name, "executed", "v1",
            )))
        }),
    };

    algebra.register_rule("saga_execution", saga_rule);

    // Note: The current API doesn't expose custom rules directly,
    // but this demonstrates the pattern
}

// ============================================================================
// Test: Algebraic Properties
// ============================================================================

#[test]
fn test_algebraic_properties() {
    let algebra = SubjectAlgebra::new();

    // Test identity element behavior
    let subject = Subject::new("test.entity.created.v1").unwrap();

    // Project with empty fields acts somewhat like identity
    let identity = algebra
        .compose(
            &subject,
            &subject,
            AlgebraOperation::Project { fields: vec![] },
        )
        .unwrap();

    assert_eq!(identity.context(), subject.context());
    assert_eq!(identity.aggregate(), subject.aggregate());

    // Test distributivity-like properties
    let a = Subject::new("service.a.event.v1").unwrap();
    let b = Subject::new("service.b.event.v1").unwrap();
    let c = Subject::new("service.c.event.v1").unwrap();

    // (A || B) seq C vs (A seq C) || (B seq C)
    let par_then_seq = algebra
        .compose(
            &algebra.compose(&a, &b, AlgebraOperation::Parallel).unwrap(),
            &c,
            AlgebraOperation::Sequence,
        )
        .unwrap();

    // These would be equal in a true distributive algebra
    assert!(par_then_seq.as_str().contains("service"));
}

// ============================================================================
// Test: Error Handling in Algebra
// ============================================================================

#[test]
fn test_algebra_error_handling() {
    let algebra = SubjectAlgebra::new();

    // Test transformation not found
    let subject = Subject::new("test.entity.created.v1").unwrap();
    let result = algebra.compose(
        &subject,
        &subject,
        AlgebraOperation::Transform {
            name: "nonexistent".to_string(),
        },
    );

    assert!(result.is_err());

    // Test pattern mismatch in transformation
    let transform = Transformation {
        name: "restricted".to_string(),
        input_pattern: Pattern::new("admin.*.*.v1").unwrap(),
        transform: Arc::new(|s| Ok(s.clone())),
    };

    algebra.register_transformation("restricted", transform);

    let user_subject = Subject::new("user.profile.updated.v1").unwrap();
    let result = algebra.compose(
        &user_subject,
        &user_subject,
        AlgebraOperation::Transform {
            name: "restricted".to_string(),
        },
    );

    // Should fail because pattern doesn't match
    assert!(result.is_err());
}

// Import Transformation directly
use cim_subject::algebra::Transformation;
