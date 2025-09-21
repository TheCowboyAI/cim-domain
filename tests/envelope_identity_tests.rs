// Copyright (c) 2025 - Cowboy AI, LLC.

use std::collections::HashMap;

use cim_domain::{
    cid::ContentType, AggregateTransactionId, CausationId, Command, CommandAcknowledgment,
    CommandEnvelope, CommandStatus, CorrelationId, DomainEvent, DomainEventEnvelope, EventId,
    MessageIdentity, PayloadMetadata, Query, QueryAcknowledgment, QueryEnvelope, QueryResponse,
    QueryStatus,
};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug)]
struct TestAggregate;

#[derive(Debug)]
struct RootCommand;

impl Command for RootCommand {
    type Aggregate = TestAggregate;

    fn aggregate_id(&self) -> Option<cim_domain::EntityId<Self::Aggregate>> {
        None
    }
}

#[derive(Debug)]
struct FollowUpCommand;

impl Command for FollowUpCommand {
    type Aggregate = TestAggregate;

    fn aggregate_id(&self) -> Option<cim_domain::EntityId<Self::Aggregate>> {
        None
    }
}

#[derive(Debug)]
struct FetchQuery;

impl Query for FetchQuery {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RaisedEvent {
    aggregate_id: Uuid,
}

impl DomainEvent for RaisedEvent {
    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn event_type(&self) -> &'static str {
        "RaisedEvent"
    }
}

fn sample_metadata() -> PayloadMetadata {
    PayloadMetadata {
        source: "tests".into(),
        version: "v1".into(),
        properties: HashMap::new(),
        payload_type: String::new(),
    }
}

#[derive(serde::Serialize)]
struct CidSource {
    id: Uuid,
}

#[test]
fn command_identity_and_event_causation_flow() {
    let root_envelope = CommandEnvelope::new(RootCommand, "system".to_string());
    let root_uuid = *root_envelope.id.as_uuid();

    match root_envelope.identity.correlation_id {
        CorrelationId::Single(id) => assert_eq!(id, root_uuid),
        other => panic!("expected single correlation id, got {other:?}"),
    }
    assert_eq!(root_envelope.identity.causation_id, CausationId(root_uuid));
    assert_eq!(root_envelope.identity.message_id, root_uuid);

    let child_envelope = CommandEnvelope::from_command(
        FollowUpCommand,
        "system".to_string(),
        &root_envelope.identity,
    );
    let child_uuid = *child_envelope.id.as_uuid();

    assert_eq!(
        child_envelope.identity.correlation_id,
        root_envelope.identity.correlation_id
    );
    assert_eq!(
        child_envelope.identity.causation_id,
        CausationId(root_envelope.identity.message_id)
    );
    assert_eq!(child_envelope.identity.message_id, child_uuid);

    let aggregate_id = Uuid::new_v4();
    let event = RaisedEvent { aggregate_id };
    let metadata = sample_metadata();
    let event_id = EventId::new();
    let event_envelope = DomainEventEnvelope::inline(
        event_id,
        event,
        child_envelope.identity.correlation_id,
        CausationId(child_envelope.identity.message_id),
        metadata,
    );

    assert_eq!(event_envelope.event_id, event_id);
    assert_eq!(event_envelope.aggregate_id, aggregate_id);
    assert_eq!(
        event_envelope.correlation_id,
        child_envelope.identity.correlation_id
    );
    assert_eq!(
        event_envelope.causation_id,
        CausationId(child_envelope.identity.message_id)
    );
    assert_eq!(event_envelope.payload_metadata.payload_type, "RaisedEvent");
    assert!(event_envelope.inline_event().is_some());
}

#[test]
fn command_envelope_new_in_tx_uses_transaction_correlation() {
    let tx = AggregateTransactionId(Uuid::new_v4());
    let envelope = CommandEnvelope::new_in_tx(RootCommand, "system".to_string(), tx);

    match envelope.identity.correlation_id {
        CorrelationId::Transaction(found) => assert_eq!(found, tx),
        other => panic!("expected transaction correlation id, got {other:?}"),
    }
    assert_eq!(
        envelope.identity.causation_id,
        CausationId(*envelope.id.as_uuid())
    );
}

#[test]
fn command_from_query_inherits_correlation() {
    let root_query = QueryEnvelope::new(FetchQuery, "system".into());
    let derived_command =
        CommandEnvelope::from_query(FollowUpCommand, "system".into(), &root_query.identity);

    assert_eq!(
        derived_command.identity.correlation_id,
        root_query.identity.correlation_id
    );
    assert_eq!(
        derived_command.identity.causation_id,
        CausationId(root_query.identity.message_id)
    );
}

#[test]
fn command_from_event_reuses_message_identity() {
    let parent_identity = MessageIdentity {
        correlation_id: CorrelationId::Single(Uuid::new_v4()),
        causation_id: CausationId(Uuid::new_v4()),
        message_id: Uuid::new_v4(),
    };
    let derived = CommandEnvelope::from_event(FollowUpCommand, "system".into(), &parent_identity);

    assert_eq!(
        derived.identity.correlation_id,
        parent_identity.correlation_id
    );
    assert_eq!(
        derived.identity.causation_id,
        CausationId(parent_identity.message_id)
    );
}

#[test]
fn query_from_command_inherits_correlation_and_sets_causation() {
    let root = CommandEnvelope::new(RootCommand, "system".into());
    let query = QueryEnvelope::from_command(FetchQuery, "system".into(), &root.identity);

    assert_eq!(query.identity.correlation_id, root.identity.correlation_id);
    assert_eq!(
        query.identity.causation_id,
        CausationId(root.identity.message_id)
    );
}

#[test]
fn query_envelope_new_in_tx_uses_transaction_correlation() {
    let tx = AggregateTransactionId(Uuid::new_v4());
    let envelope = QueryEnvelope::new_in_tx(FetchQuery, "system".into(), tx);

    match envelope.identity.correlation_id {
        CorrelationId::Transaction(found) => assert_eq!(found, tx),
        other => panic!("expected transaction correlation id, got {other:?}"),
    }
    assert_eq!(
        envelope.identity.causation_id,
        CausationId(*envelope.id.as_uuid())
    );
}

#[test]
fn query_from_event_reuses_message_identity() {
    let parent_identity = MessageIdentity {
        correlation_id: CorrelationId::Single(Uuid::new_v4()),
        causation_id: CausationId(Uuid::new_v4()),
        message_id: Uuid::new_v4(),
    };
    let derived = QueryEnvelope::from_event(FetchQuery, "system".into(), &parent_identity);

    assert_eq!(
        derived.identity.correlation_id,
        parent_identity.correlation_id
    );
    assert_eq!(
        derived.identity.causation_id,
        CausationId(parent_identity.message_id)
    );
}

#[test]
fn event_id_generation_is_time_ordered() {
    let ids: Vec<EventId> = (0..128).map(|_| EventId::new()).collect();
    for window in ids.windows(2) {
        let prev = window[0].0;
        let next = window[1].0;
        assert!(prev <= next, "event ids must be non-decreasing");
    }
}

#[test]
fn event_envelope_fields_preserved_when_swapping_payload() {
    let correlation = CorrelationId::Single(Uuid::new_v4());
    let causation = CausationId(Uuid::new_v4());
    let aggregate_id = Uuid::new_v4();
    let mut metadata = sample_metadata();
    metadata
        .properties
        .insert("key".into(), serde_json::json!("value"));

    let event = RaisedEvent { aggregate_id };
    let env = DomainEventEnvelope::inline(EventId::new(), event, correlation, causation, metadata);
    assert!(env.inline_event().is_some());
    assert_eq!(env.payload_metadata.payload_type, "RaisedEvent");

    let cid_source = CidSource { id: Uuid::new_v4() };
    let cid = cim_domain::cid::generate_cid(&cid_source, ContentType::Raw).unwrap();
    let swapped = env.with_payload_cid(cid.clone());

    assert!(swapped.inline_event().is_none());
    assert_eq!(swapped.payload_cid(), Some(&cid));
    assert_eq!(swapped.aggregate_id, aggregate_id);
    assert_eq!(swapped.correlation_id, correlation);
    assert_eq!(swapped.causation_id, causation);
    assert_eq!(
        swapped.payload_metadata.properties.get("key"),
        Some(&serde_json::json!("value"))
    );
}

#[test]
fn command_acknowledgment_mirrors_envelope_identity() {
    let root = CommandEnvelope::new(RootCommand, "system".into());
    let ack = CommandAcknowledgment {
        command_id: root.id,
        correlation_id: root.identity.correlation_id.clone(),
        status: CommandStatus::Accepted,
        reason: None,
    };

    assert_eq!(ack.command_id, root.id);
    assert_eq!(ack.correlation_id, root.identity.correlation_id);
    assert_eq!(ack.status, CommandStatus::Accepted);
}

#[test]
fn query_acknowledgment_statuses() {
    let root_query = QueryEnvelope::new(FetchQuery, "user".into());
    let accepted = QueryAcknowledgment {
        query_id: root_query.id,
        correlation_id: root_query.identity.correlation_id.clone(),
        status: QueryStatus::Accepted,
        reason: None,
    };
    assert_eq!(accepted.status, QueryStatus::Accepted);
    assert!(accepted.reason.is_none());

    let rejected = QueryAcknowledgment {
        query_id: root_query.id,
        correlation_id: root_query.identity.correlation_id.clone(),
        status: QueryStatus::Rejected,
        reason: Some("invalid filter".into()),
    };
    assert_eq!(rejected.status, QueryStatus::Rejected);
    assert_eq!(rejected.reason.as_deref(), Some("invalid filter"));
}

#[test]
fn query_response_carries_identity_and_payload() {
    let correlation = CorrelationId::Single(Uuid::new_v4());
    let response = QueryResponse {
        query_id: Uuid::new_v4(),
        correlation_id: correlation.clone(),
        result: json!({"ok": true}),
    };

    assert_eq!(response.correlation_id, correlation);
    assert_eq!(response.result["ok"], json!(true));
}
