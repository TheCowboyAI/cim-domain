// Copyright 2025 Cowboy AI, LLC.

//! Domain events for CIM
//!
//! Events represent facts that have occurred in the domain. They are immutable
//! and form the basis of event sourcing and event-driven communication.

use crate::cid::DomainCid;
use crate::fp_adts::Either;
use crate::cqrs::{CausationId, CorrelationId, EventId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// PropagationScope removed - infrastructure concern, belongs in transport layer
// EventEnvelope removed - has routing concerns, belongs in cim-subject or transport layer

/// Base trait for all domain events
///
/// # Examples
///
/// ```rust
/// use cim_domain::DomainEvent;
/// use uuid::Uuid;
///
/// #[derive(Debug)]
/// struct UserCreatedEvent {
///     user_id: Uuid,
///     email: String,
///     created_at: std::time::SystemTime,
/// }
///
/// impl DomainEvent for UserCreatedEvent {
///     fn aggregate_id(&self) -> Uuid {
///         self.user_id
///     }
///     
///     fn event_type(&self) -> &'static str {
///         "UserCreated"
///     }
///     
///     fn version(&self) -> &'static str {
///         "v1"
///     }
/// }
///
/// let event = UserCreatedEvent {
///     user_id: Uuid::new_v4(),
///     email: "user@example.com".to_string(),
///     created_at: std::time::SystemTime::now(),
/// };
///
/// assert_eq!(event.event_type(), "UserCreated");
/// ```
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// Get the aggregate ID this event relates to
    fn aggregate_id(&self) -> Uuid;

    /// Get the event type name
    fn event_type(&self) -> &'static str;

    /// Get the schema version
    fn version(&self) -> &'static str {
        "v1"
    }
}

/// How an event payload is represented inside an envelope.
// Payload is modeled as an Either: Left(DomainCid) | Right(Event)

/// Domain event envelope carrying identity and either an inline payload or a CID reference.
///
/// This type is pure and performs no persistence; infrastructure is responsible for
/// extracting `Inline(E)`, producing a `DomainCid`, and replacing the payload with `ByCid(DomainCid)`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DomainEventEnvelope<E: DomainEvent> {
    /// The event ID
    pub event_id: EventId,

    /// Aggregate identifier copied from the event so it remains available after CID substitution
    pub aggregate_id: uuid::Uuid,

    /// Correlation ID for tracking across services
    pub correlation_id: CorrelationId,

    /// ID of the event that caused this one
    pub causation_id: CausationId,

    /// Metadata that describes the payload (schema/source/properties)
    pub payload_metadata: PayloadMetadata,

    /// Event payload, either inline or by CID
    pub payload: Either<DomainCid, E>,
}

/// Metadata that describes the payload (not the event itself)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PayloadMetadata {
    /// Source or schema namespace for the payload
    pub source: String,

    /// Payload schema or version tag
    pub version: String,

    /// Additional metadata properties (payload-oriented)
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

// DomainEventEnvelope removed - had subject routing which belongs in transport layer

impl<E: DomainEvent> DomainEventEnvelope<E> {
    /// Construct an envelope with an inline event payload (pre-persist).
    pub fn inline(
        event_id: EventId,
        event: E,
        correlation_id: CorrelationId,
        causation_id: CausationId,
        payload_metadata: PayloadMetadata,
    ) -> Self {
        let aggregate_id = event.aggregate_id();
        Self {
            event_id,
            aggregate_id,
            correlation_id,
            causation_id,
            payload_metadata,
            payload: Either::Right(event),
        }
    }

    /// Construct an envelope that references the event payload by CID (post-persist).
    pub fn by_cid(
        event_id: EventId,
        aggregate_id: uuid::Uuid,
        correlation_id: CorrelationId,
        causation_id: CausationId,
        payload_metadata: PayloadMetadata,
        cid: DomainCid,
    ) -> Self {
        Self {
            event_id,
            aggregate_id,
            correlation_id,
            causation_id,
            payload_metadata,
            payload: Either::Left(cid),
        }
    }

    /// Replace an inline payload with a CID reference, keeping all metadata.
    pub fn with_payload_cid(self, cid: DomainCid) -> Self {
        Self { payload: Either::Left(cid), ..self }
    }

    /// Accessor: return inline event reference if present.
    pub fn inline_event(&self) -> Option<&E> {
        match &self.payload {
            Either::Right(e) => Some(e),
            _ => None,
        }
    }

    /// Accessor: return CID reference if present.
    pub fn payload_cid(&self) -> Option<&DomainCid> {
        match &self.payload {
            Either::Left(c) => Some(c),
            _ => None,
        }
    }
}

// All domain-specific events have been moved to their respective domain submodules:
// - Person events: cim-domain-person
// - Organization events: cim-domain-organization
// - Agent events: cim-domain-agent
// - Workflow events: cim-domain-workflow
// - Location events: cim-domain-location
// - Document events: cim-domain-document
// - Policy events: cim-domain-policy

// Subject/propagation tests removed; transport concerns live downstream

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct TestEvent {
        id: Uuid,
        name: String,
    }

    impl DomainEvent for TestEvent {
        fn aggregate_id(&self) -> Uuid { self.id }
        fn event_type(&self) -> &'static str { "TestEvent" }
        fn version(&self) -> &'static str { "v1" }
    }

    #[test]
    fn event_envelope_inline_and_cid() {
        let event = TestEvent { id: Uuid::new_v4(), name: "created".into() };
        let eid = Uuid::new_v4();
        let env = DomainEventEnvelope::inline(
            crate::cqrs::EventId(eid),
            event.clone(),
            CorrelationId::Single(Uuid::new_v4()),
            CausationId(Uuid::new_v4()),
            PayloadMetadata { source: "tests".into(), version: "v1".into(), properties: Default::default() },
        );
        assert_eq!(env.event_id, crate::cqrs::EventId(eid));
        assert_eq!(env.aggregate_id, event.id);
        assert!(env.inline_event().is_some());

        // Simulate persistence by swapping payload for a CID
        let cid = crate::cid::generate_cid(&event, crate::cid::ContentType::Event).unwrap();
        let env2 = env.with_payload_cid(cid.clone());
        assert!(env2.inline_event().is_none());
        assert_eq!(env2.payload_cid().cloned(), Some(cid));
    }

    #[test]
    fn event_envelope_serde_roundtrip() {
        let event = TestEvent { id: Uuid::new_v4(), name: "updated".into() };
        let env = DomainEventEnvelope::inline(
            crate::cqrs::EventId(Uuid::new_v4()),
            event,
            CorrelationId::Single(Uuid::new_v4()),
            CausationId(Uuid::new_v4()),
            PayloadMetadata { source: "tests".into(), version: "v1".into(), properties: Default::default() },
        );
        let json = serde_json::to_string(&env).unwrap();
        let back: DomainEventEnvelope<TestEvent> = serde_json::from_str(&json).unwrap();
        assert_eq!(env.event_id, back.event_id);
        assert_eq!(env.aggregate_id, back.aggregate_id);
        assert_eq!(env.correlation_id, back.correlation_id);
        assert_eq!(env.causation_id, back.causation_id);
        assert_eq!(env.payload_metadata.source, back.payload_metadata.source);
        assert!(back.payload.right().is_some());
    }
}
