// Copyright 2025 Cowboy AI, LLC.

//! # CQRS (Command Query Responsibility Segregation) Pattern
//!
//! This module provides the foundational types and traits for implementing CQRS
//! in a domain-driven design context. Commands represent write operations that
//! modify state, while queries represent read operations that retrieve data.

use crate::entity::EntityId;
use crate::markers::{CommandMarker, QueryMarker};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use uuid::Uuid;

/// Correlation ID for tracking related commands and events.
///
/// Math: correlation_id = (message_id | aggregate_transaction_id)
/// - Single: non-transactional, single morphism; correlation == message_id
/// - Transaction: transactional; correlation == aggregate transaction id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value")]
pub enum CorrelationId {
    /// Non-transactional, correlation equals message_id
    Single(Uuid),
    /// Transactional, correlation equals aggregate transaction id
    Transaction(AggregateTransactionId),
}

// No default; correlation must be provided explicitly

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorrelationId::Single(id) => write!(f, "correlation:{}", id),
            CorrelationId::Transaction(tx) => write!(f, "correlation:{}", tx.0),
        }
    }
}

/// Causation ID for tracking event causality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct CausationId(pub Uuid);

// No default; causation must be provided explicitly

impl fmt::Display for CausationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "causation:{}", self.0)
    }
}

/// Type alias for ID types
pub type IdType = Uuid;

/// Aggregate Transaction identifier (provides correlation IDs for transactions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct AggregateTransactionId(pub Uuid);

impl From<AggregateTransactionId> for CorrelationId {
    fn from(tx: AggregateTransactionId) -> Self {
        CorrelationId::Transaction(tx)
    }
}

/// Factory for creating message identities
pub struct MessageFactory;

impl MessageFactory {
    /// Create a non-transactional root identity (single morphism):
    /// correlation == causation == message id
    pub fn create_root_command(id: Uuid) -> MessageIdentity {
        MessageIdentity {
            correlation_id: CorrelationId::Single(id),
            causation_id: CausationId(id),
            message_id: id,
        }
    }

    /// Create a transactional root identity (correlation comes from tx).
    /// Root causation references its own message_id (no prior cause).
    pub fn create_root_command_in_tx(id: Uuid, tx: AggregateTransactionId) -> MessageIdentity {
        MessageIdentity {
            correlation_id: tx.into(),
            causation_id: CausationId(id),
            message_id: id,
        }
    }

    pub fn command_from_command(id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
        MessageIdentity {
            correlation_id: parent.correlation_id,
            causation_id: CausationId(parent.message_id),
            message_id: id,
        }
    }

    pub fn command_from_query(id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
        MessageIdentity {
            correlation_id: parent.correlation_id,
            causation_id: CausationId(parent.message_id),
            message_id: id,
        }
    }

    pub fn command_from_event(id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
        MessageIdentity {
            correlation_id: parent.correlation_id,
            causation_id: CausationId(parent.message_id),
            message_id: id,
        }
    }

    /// Create a non-transactional root identity (single morphism) for a query
    pub fn create_root_query(id: Uuid) -> MessageIdentity {
        MessageIdentity {
            correlation_id: CorrelationId::Single(id),
            causation_id: CausationId(id),
            message_id: id,
        }
    }

    /// Create a transactional root identity for a query
    pub fn create_root_query_in_tx(id: Uuid, tx: AggregateTransactionId) -> MessageIdentity {
        MessageIdentity {
            correlation_id: tx.into(),
            causation_id: CausationId(id),
            message_id: id,
        }
    }

    pub fn query_from_command(id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
        MessageIdentity {
            correlation_id: parent.correlation_id,
            causation_id: CausationId(parent.message_id),
            message_id: id,
        }
    }

    pub fn query_from_query(id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
        MessageIdentity {
            correlation_id: parent.correlation_id,
            causation_id: CausationId(parent.message_id),
            message_id: id,
        }
    }

    pub fn query_from_event(id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
        MessageIdentity {
            correlation_id: parent.correlation_id,
            causation_id: CausationId(parent.message_id),
            message_id: id,
        }
    }
}

/// Message identity for tracking message metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MessageIdentity {
    pub correlation_id: CorrelationId,
    pub causation_id: CausationId,
    pub message_id: Uuid,
}

// No default; identities must be constructed explicitly via factory.

/// Status of command acceptance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CommandStatus {
    /// Command was accepted for processing
    Accepted,
    /// Command was rejected (e.g., validation failed)
    Rejected,
}

/// Status of query acceptance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum QueryStatus {
    /// Query was accepted for processing
    Accepted,
    /// Query was rejected (e.g., invalid parameters)
    Rejected,
}

/// Acknowledgment returned when a command is submitted
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommandAcknowledgment {
    /// The command ID that was acknowledged
    pub command_id: CommandId,
    /// Correlation ID (same as command ID for originating commands)
    pub correlation_id: CorrelationId,
    /// Status of command acceptance
    pub status: CommandStatus,
    /// Optional rejection reason
    pub reason: Option<String>,
}

/// Query acknowledgment returned by query handlers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAcknowledgment {
    /// The query ID that was acknowledged
    pub query_id: QueryId,
    /// Correlation ID (same as query ID for originating queries)
    pub correlation_id: CorrelationId,
    /// Status of query acceptance
    pub status: QueryStatus,
    /// Optional rejection reason
    pub reason: Option<String>,
}

/// Query response returned by query handlers
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryResponse {
    /// The query ID that was processed
    pub query_id: IdType,
    /// Correlation ID for tracking
    pub correlation_id: CorrelationId,
    /// The result data
    pub result: serde_json::Value,
}

/// A command that requests a state change
///
/// Commands are write operations that modify state. They should be named
/// with imperative verbs (CreateOrder, UpdateCustomer, DeleteProduct).
///
/// Commands do NOT return results directly - results come through event streams.
///
/// # Examples
///
/// ```rust
/// use cim_domain::{Command, EntityId};
///
/// #[derive(Debug)]
/// struct CreateOrderCommand {
///     customer_id: String,
///     items: Vec<OrderItem>,
/// }
///
/// #[derive(Debug)]
/// struct OrderItem {
///     product_id: String,
///     quantity: u32,
/// }
///
/// struct Order;
///
/// impl Command for CreateOrderCommand {
///     type Aggregate = Order;
///     
///     fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
///         None // New order, no ID yet
///     }
/// }
///
/// let cmd = CreateOrderCommand {
///     customer_id: "CUST-123".to_string(),
///     items: vec![
///         OrderItem {
///             product_id: "PROD-456".to_string(),
///             quantity: 2,
///         }
///     ],
/// };
///
/// // Commands are sent to handlers, which validate and emit events
/// assert!(cmd.aggregate_id().is_none());
/// ```
pub trait Command: Debug + Send + Sync {
    /// The aggregate type this command targets
    type Aggregate;

    /// Get the aggregate ID this command targets
    fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>>;
}

/// A query that requests data without modifying state
///
/// Queries are read operations that return data. They should be named
/// to describe what they return (GetOrderById, FindCustomersByRegion).
///
/// Queries do NOT return results directly - results come through event streams.
pub trait Query: Debug + Send + Sync {
    // Queries don't need additional methods beyond Debug + Send + Sync
}

/// Type alias for command IDs
pub type CommandId = EntityId<CommandMarker>;

/// Type alias for query IDs
pub type QueryId = EntityId<QueryMarker>;

/// Type alias for event IDs (using CID)
/// Event ID - UUID v7 for time-ordered event identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct EventId(pub Uuid);

impl EventId {
    /// Create a new EventId with UUID v7 (time-ordered)
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A command with metadata for tracking and auditing
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommandEnvelope<C> {
    /// Unique identifier for this command instance
    pub id: CommandId,
    /// The actual command
    pub command: C,
    /// Who issued this command
    pub issued_by: String,
    /// Message identity (correlation and causation)
    pub identity: MessageIdentity,
}

impl<C: Command> CommandEnvelope<C> {
    /// Create a new command envelope within an aggregate transaction (correlation provided).
    pub fn new_in_tx(command: C, issued_by: String, tx: AggregateTransactionId) -> Self {
        let id = CommandId::new();
        let identity = MessageFactory::create_root_command_in_tx(*id.as_uuid(), tx);

        Self {
            id,
            command,
            issued_by,
            identity,
        }
    }

    /// Create a non-transactional (single morphism) command envelope.
    pub fn new(command: C, issued_by: String) -> Self {
        let id = CommandId::new();
        let identity = MessageFactory::create_root_command(*id.as_uuid());
        Self {
            id,
            command,
            issued_by,
            identity,
        }
    }

    /// Create a command caused by another command (continues correlation)
    pub fn from_command(command: C, issued_by: String, parent_identity: &MessageIdentity) -> Self {
        let id = CommandId::new();
        let identity = MessageFactory::command_from_command(*id.as_uuid(), parent_identity);

        Self {
            id,
            command,
            issued_by,
            identity,
        }
    }

    /// Create a command caused by a query (continues correlation)
    pub fn from_query(command: C, issued_by: String, parent_identity: &MessageIdentity) -> Self {
        let id = CommandId::new();
        let identity = MessageFactory::command_from_query(*id.as_uuid(), parent_identity);

        Self {
            id,
            command,
            issued_by,
            identity,
        }
    }

    /// Create a command caused by an event (continues correlation)
    pub fn from_event(command: C, issued_by: String, parent_identity: &MessageIdentity) -> Self {
        let id = CommandId::new();
        let identity = MessageFactory::command_from_event(*id.as_uuid(), parent_identity);

        Self {
            id,
            command,
            issued_by,
            identity,
        }
    }

    /// Get the correlation ID
    pub fn correlation_id(&self) -> &CorrelationId {
        &self.identity.correlation_id
    }

    /// Get the causation ID (if any)
    pub fn causation_id(&self) -> &CausationId {
        &self.identity.causation_id
    }
}

/// A query with metadata for tracking
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryEnvelope<Q> {
    /// Unique identifier for this query instance
    pub id: QueryId,
    /// The actual query
    pub query: Q,
    /// Who issued this query
    pub issued_by: String,
    /// Message identity (correlation and causation)
    pub identity: MessageIdentity,
}

impl<Q: Query> QueryEnvelope<Q> {
    /// Create a new query envelope within an aggregate transaction (correlation provided).
    pub fn new_in_tx(query: Q, issued_by: String, tx: AggregateTransactionId) -> Self {
        let id = QueryId::new();
        let identity = MessageFactory::create_root_query_in_tx(*id.as_uuid(), tx);

        Self {
            id,
            query,
            issued_by,
            identity,
        }
    }

    /// Create a non-transactional (single morphism) query envelope.
    pub fn new(query: Q, issued_by: String) -> Self {
        let id = QueryId::new();
        let identity = MessageFactory::create_root_query(*id.as_uuid());
        Self {
            id,
            query,
            issued_by,
            identity,
        }
    }

    /// Create a query caused by a command (continues correlation)
    pub fn from_command(query: Q, issued_by: String, parent_identity: &MessageIdentity) -> Self {
        let id = QueryId::new();
        let identity = MessageFactory::query_from_command(*id.as_uuid(), parent_identity);

        Self {
            id,
            query,
            issued_by,
            identity,
        }
    }

    /// Create a query caused by another query (continues correlation)
    pub fn from_query(query: Q, issued_by: String, parent_identity: &MessageIdentity) -> Self {
        let id = QueryId::new();
        let identity = MessageFactory::query_from_query(*id.as_uuid(), parent_identity);

        Self {
            id,
            query,
            issued_by,
            identity,
        }
    }

    /// Create a query caused by an event (continues correlation)
    pub fn from_event(query: Q, issued_by: String, parent_identity: &MessageIdentity) -> Self {
        let id = QueryId::new();
        let identity = MessageFactory::query_from_event(*id.as_uuid(), parent_identity);

        Self {
            id,
            query,
            issued_by,
            identity,
        }
    }

    /// Get the correlation ID
    pub fn correlation_id(&self) -> &CorrelationId {
        &self.identity.correlation_id
    }

    /// Get the causation ID (if any)
    pub fn causation_id(&self) -> &CausationId {
        &self.identity.causation_id
    }
}

/// Handler for processing commands
///
/// Handlers return only acknowledgments. Results are published to event streams.
pub trait CommandHandler<C: Command> {
    /// Handle the command and return acknowledgment
    fn handle(&mut self, envelope: CommandEnvelope<C>) -> CommandAcknowledgment;
}

/// Handler for processing queries
///
/// Handlers return query responses with the result data.
pub trait QueryHandler<Q: Query> {
    /// Handle the query and return response
    fn handle(&self, envelope: QueryEnvelope<Q>) -> QueryResponse;
}

/// Event stream subscription for receiving command/query results
#[derive(Debug, Clone)]
pub struct EventStreamSubscription {
    /// Stream name to subscribe to
    pub stream_name: String,
    /// Filter for specific correlation IDs (None = all)
    pub correlation_filter: Option<CorrelationId>,
    /// Filter for specific causation IDs (None = all)
    pub causation_filter: Option<CausationId>,
}

impl EventStreamSubscription {
    /// Create a subscription for a specific correlation
    pub fn for_correlation(stream_name: String, correlation_id: CorrelationId) -> Self {
        Self {
            stream_name,
            correlation_filter: Some(correlation_id),
            causation_filter: None,
        }
    }

    /// Create a subscription for events caused by a specific message
    pub fn for_causation(stream_name: String, causation_id: CausationId) -> Self {
        Self {
            stream_name,
            correlation_filter: None,
            causation_filter: Some(causation_id),
        }
    }

    /// Create a subscription for all events on a stream
    pub fn for_all(stream_name: String) -> Self {
        Self {
            stream_name,
            correlation_filter: None,
            causation_filter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::AggregateMarker;
    use uuid::Uuid;

    /// Test command for testing
    #[derive(Debug, Clone)]
    struct TestCommand {
        _name: String,
        aggregate_id: Option<EntityId<AggregateMarker>>,
    }

    impl Command for TestCommand {
        type Aggregate = AggregateMarker;

        fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
            self.aggregate_id
        }
    }

    /// Test query for testing
    #[derive(Debug, Clone)]
    struct TestQuery {
        _filter: String,
    }

    impl Query for TestQuery {}

    /// Test the flow of command creation and correlation
    ///
    /// ```mermaid
    /// graph LR
    ///     A[User Action] -->|Creates| B[Command]
    ///     B -->|Self-reference| C[CorrelationId]
    ///     B -->|Self-reference| D[CausationId]
    /// ```
    #[test]
    fn test_command_envelope_new() {
        let command = TestCommand {
            _name: "test".to_string(),
            aggregate_id: Some(EntityId::new()),
        };

        let tx = AggregateTransactionId(Uuid::new_v4());
        let envelope = CommandEnvelope::new_in_tx(command.clone(), "user123".to_string(), tx);

        // Verify basic properties
        assert_eq!(envelope.issued_by, "user123");

        // Root message: correlation from tx; causation from message id
        match envelope.identity.correlation_id {
            CorrelationId::Transaction(t) => assert_eq!(t.0, tx.0),
            _ => panic!("expected transactional correlation"),
        }
        assert_eq!(envelope.identity.causation_id.0, *envelope.id.as_uuid());
    }

    /// Test command caused by another command
    ///
    /// ```mermaid
    /// graph LR
    ///     A[Command1] -->|Causes| B[Command2]
    ///     A -->|Shares| C[CorrelationId]
    ///     C -->|With| B
    ///     A -->|Referenced by| D[CausationId]
    ///     D -->|In| B
    /// ```
    #[test]
    fn test_command_envelope_from_command() {
        // Create parent command
        let parent_command = TestCommand {
            _name: "parent".to_string(),
            aggregate_id: None,
        };
        let tx = AggregateTransactionId(Uuid::new_v4());
        let parent_envelope = CommandEnvelope::new_in_tx(parent_command, "user".to_string(), tx);

        // Create child command
        let child_command = TestCommand {
            _name: "child".to_string(),
            aggregate_id: None,
        };

        let child_envelope = CommandEnvelope::from_command(
            child_command,
            "system".to_string(),
            &parent_envelope.identity,
        );

        // Verify causation points to parent message id
        assert_eq!(
            child_envelope.identity.causation_id.0,
            *parent_envelope.id.as_uuid()
        );

        // Verify correlation is preserved from parent
        assert_eq!(
            child_envelope.identity.correlation_id,
            parent_envelope.identity.correlation_id
        );
    }

    /// Test query envelope creation
    ///
    /// ```mermaid
    /// graph LR
    ///     A[User Query] -->|Creates| B[QueryEnvelope]
    ///     B -->|Self-reference| C[CorrelationId]
    ///     B -->|Self-reference| D[CausationId]
    /// ```
    #[test]
    fn test_query_envelope_new() {
        let query = TestQuery {
            _filter: "active".to_string(),
        };

        let tx = AggregateTransactionId(Uuid::new_v4());
        let envelope = QueryEnvelope::new_in_tx(query, "user456".to_string(), tx);

        // Verify basic properties
        assert_eq!(envelope.issued_by, "user456");

        // Root query: correlation from tx; causation from message id
        match envelope.identity.correlation_id {
            CorrelationId::Transaction(t) => assert_eq!(t.0, tx.0),
            _ => panic!("expected transactional correlation"),
        }
        assert_eq!(envelope.identity.causation_id.0, *envelope.id.as_uuid());
    }

    /// Test query caused by event
    ///
    /// ```mermaid
    /// graph LR
    ///     A[Event] -->|Causes| B[Query]
    ///     A -->|CID| C[CausationId]
    ///     C -->|In| B
    ///     D[CorrelationId] -->|Preserved in| B
    /// ```
    #[test]
    fn test_query_envelope_from_event() {
        // Create a mock event identity with UUIDs only (CIDs are payload-only)
        let event_identity = MessageFactory::create_root_command(Uuid::new_v4());

        let query = TestQuery {
            _filter: "by-event".to_string(),
        };

        let envelope =
            QueryEnvelope::from_event(query, "event-handler".to_string(), &event_identity);

        // Verify causation points to event message_id
        assert_eq!(envelope.identity.causation_id.0, event_identity.message_id);

        // Verify correlation is preserved
        assert_eq!(
            envelope.identity.correlation_id,
            event_identity.correlation_id
        );
    }

    /// Test correlation ID display formats
    #[test]
    fn test_correlation_id_display() {
        let command_id = CommandId::new();
        let correlation = CorrelationId::Single(*command_id.as_uuid());
        let display = format!("{correlation}");
        assert!(display.starts_with("correlation:"));
        assert!(display.contains(&command_id.as_uuid().to_string()));
    }

    /// Test causation ID display formats
    #[test]
    fn test_causation_id_display() {
        let query_id = QueryId::new();
        let causation = CausationId(*query_id.as_uuid());
        let display = format!("{causation}");
        assert!(display.starts_with("causation:"));
        assert!(display.contains(&query_id.as_uuid().to_string()));
    }

    /// Test command acknowledgment creation
    #[test]
    fn test_command_acknowledgment() {
        let command_id = CommandId::new();
        let correlation_id = CorrelationId::Single(*command_id.as_uuid());

        let ack = CommandAcknowledgment {
            command_id,
            correlation_id: correlation_id.clone(),
            status: CommandStatus::Accepted,
            reason: None,
        };

        assert_eq!(ack.status, CommandStatus::Accepted);
        assert!(ack.reason.is_none());

        let rejected_ack = CommandAcknowledgment {
            command_id,
            correlation_id,
            status: CommandStatus::Rejected,
            reason: Some("Validation failed".to_string()),
        };

        assert_eq!(rejected_ack.status, CommandStatus::Rejected);
        assert_eq!(rejected_ack.reason, Some("Validation failed".to_string()));
    }

    /// Test event stream subscription patterns
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Event Stream] -->|Filter by| B[Correlation]
    ///     A -->|Filter by| C[Causation]
    ///     A -->|No filter| D[All Events]
    /// ```
    #[test]
    fn test_event_stream_subscription() {
        let correlation_id = CorrelationId::Single(Uuid::new_v4());
        let causation_id = CausationId(Uuid::new_v4());

        // Test correlation filter
        let sub1 = EventStreamSubscription::for_correlation(
            "test-stream".to_string(),
            correlation_id.clone(),
        );
        assert_eq!(sub1.stream_name, "test-stream");
        assert_eq!(sub1.correlation_filter, Some(correlation_id));
        assert!(sub1.causation_filter.is_none());

        // Test causation filter
        let sub2 =
            EventStreamSubscription::for_causation("test-stream".to_string(), causation_id.clone());
        assert_eq!(sub2.stream_name, "test-stream");
        assert!(sub2.correlation_filter.is_none());
        assert_eq!(sub2.causation_filter, Some(causation_id));

        // Test no filter
        let sub3 = EventStreamSubscription::for_all("test-stream".to_string());
        assert_eq!(sub3.stream_name, "test-stream");
        assert!(sub3.correlation_filter.is_none());
        assert!(sub3.causation_filter.is_none());
    }

    /// Test command handler trait implementation
    struct TestCommandHandler {
        accepted_count: std::cell::RefCell<usize>,
    }

    impl CommandHandler<TestCommand> for TestCommandHandler {
        fn handle(&mut self, envelope: CommandEnvelope<TestCommand>) -> CommandAcknowledgment {
            *self.accepted_count.borrow_mut() += 1;

            CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id().clone(),
                status: CommandStatus::Accepted,
                reason: None,
            }
        }
    }

    #[test]
    fn test_command_handler() {
        let mut handler = TestCommandHandler {
            accepted_count: std::cell::RefCell::new(0),
        };

        let command = TestCommand {
            _name: "test".to_string(),
            aggregate_id: None,
        };

        let tx = AggregateTransactionId(Uuid::new_v4());
        let envelope = CommandEnvelope::new_in_tx(command, "user".to_string(), tx);
        let ack = handler.handle(envelope.clone());

        assert_eq!(ack.command_id, envelope.id);
        assert_eq!(ack.status, CommandStatus::Accepted);
        assert_eq!(*handler.accepted_count.borrow(), 1);
    }

    #[test]
    fn event_id_is_monotonic_non_decreasing_by_bytes() {
        // Generate a short sequence of EventIds and assert pairwise non-decreasing by bytes
        let mut ids: Vec<EventId> = Vec::new();
        for _ in 0..12 { ids.push(EventId::new()); }
        for w in ids.windows(2) {
            let a = (w[0].0).as_bytes();
            let b = (w[1].0).as_bytes();
            assert!(a <= b, "EventId must be non-decreasing: {:?} <= {:?}", a, b);
        }
    }
}
