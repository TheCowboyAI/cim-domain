//! Command Query Responsibility Segregation (CQRS) types
//!
//! In CIM's event-driven architecture:
//! - Commands and Queries return only acknowledgments
//! - Results are delivered through event streams
//! - All interactions are asynchronous via events
//! - Correlation tracks related messages (defaults to self-reference)
//! - Causation must reference existing messages

use crate::entity::EntityId;
use crate::markers::{CommandMarker, QueryMarker};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display};
use cid::Cid;
use uuid::Uuid;

/// ID type that can be either a CID or UUID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdType {
    /// UUID for commands and queries
    Uuid(Uuid),
    /// Content-addressed ID for events
    Cid(Cid),
}

impl Display for IdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdType::Uuid(uuid) => write!(f, "{}", uuid),
            IdType::Cid(cid) => write!(f, "{}", cid),
        }
    }
}

/// Unique identifier for correlating related messages
///
/// For the first message in a correlation chain, this is a self-reference.
/// All subsequent messages in the chain share the same correlation ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(pub IdType);

impl CorrelationId {
    /// Create a correlation ID from a command (self-reference for new correlations)
    pub fn from_command(command_id: CommandId) -> Self {
        Self(IdType::Uuid(*command_id.as_uuid()))
    }

    /// Create a correlation ID from a query (self-reference for new correlations)
    pub fn from_query(query_id: QueryId) -> Self {
        Self(IdType::Uuid(*query_id.as_uuid()))
    }

    /// Create a correlation ID from an event (self-reference for new correlations)
    pub fn from_event(event_cid: Cid) -> Self {
        Self(IdType::Cid(event_cid))
    }
}

impl Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "correlation:{}", self.0)
    }
}

/// Identifies what caused this message to be created
///
/// This MUST reference an existing message that has already been processed.
/// Only messages that are caused by other messages have a causation ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CausationId(pub IdType);

impl CausationId {
    /// Create a causation ID from an existing command
    pub fn from_command(command_id: CommandId) -> Self {
        Self(IdType::Uuid(*command_id.as_uuid()))
    }

    /// Create a causation ID from an existing query
    pub fn from_query(query_id: QueryId) -> Self {
        Self(IdType::Uuid(*query_id.as_uuid()))
    }

    /// Create a causation ID from an existing event
    pub fn from_event(event_cid: Cid) -> Self {
        Self(IdType::Cid(event_cid))
    }
}

impl Display for CausationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "causation:{}", self.0)
    }
}

/// Status of command acceptance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandStatus {
    /// Command was accepted for processing
    Accepted,
    /// Command was rejected (e.g., validation failed)
    Rejected,
}

/// Status of query acceptance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    /// Query was accepted for processing
    Accepted,
    /// Query was rejected (e.g., invalid parameters)
    Rejected,
}

/// Acknowledgment returned when a command is submitted
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Acknowledgment returned when a query is submitted
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// A command that requests a state change
///
/// Commands are write operations that modify state. They should be named
/// with imperative verbs (CreateOrder, UpdateCustomer, DeleteProduct).
///
/// Commands do NOT return results directly - results come through event streams.
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
pub type EventId = Cid;

/// A command with metadata for tracking and auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope<C> {
    /// Unique identifier for this command instance
    pub id: CommandId,
    /// The actual command
    pub command: C,
    /// Who issued this command
    pub issued_by: String,
    /// Correlation ID - for new commands, this is self-reference
    pub correlation_id: CorrelationId,
    /// What caused this command (None for user-initiated commands)
    pub causation_id: Option<CausationId>,
}

impl<C: Command> CommandEnvelope<C> {
    /// Create a new command envelope (user-initiated, starts new correlation)
    pub fn new(command: C, issued_by: String) -> Self {
        let id = CommandId::new();
        // New correlation chain - self-reference
        let correlation_id = CorrelationId::from_command(id);

        Self {
            id,
            command,
            issued_by,
            correlation_id,
            causation_id: None, // User-initiated has no causation
        }
    }

    /// Create a command caused by another command (continues correlation)
    pub fn from_command(
        command: C,
        issued_by: String,
        causing_command_id: CommandId,
        correlation: CorrelationId,
    ) -> Self {
        let id = CommandId::new();

        Self {
            id,
            command,
            issued_by,
            correlation_id: correlation, // Continue existing correlation
            causation_id: Some(CausationId::from_command(causing_command_id)),
        }
    }

    /// Create a command caused by a query (continues correlation)
    pub fn from_query(
        command: C,
        issued_by: String,
        causing_query_id: QueryId,
        correlation: CorrelationId,
    ) -> Self {
        let id = CommandId::new();

        Self {
            id,
            command,
            issued_by,
            correlation_id: correlation, // Continue existing correlation
            causation_id: Some(CausationId::from_query(causing_query_id)),
        }
    }

    /// Create a command caused by an event (continues correlation)
    pub fn from_event(
        command: C,
        issued_by: String,
        causing_event_cid: Cid,
        correlation: CorrelationId,
    ) -> Self {
        let id = CommandId::new();

        Self {
            id,
            command,
            issued_by,
            correlation_id: correlation, // Continue existing correlation
            causation_id: Some(CausationId::from_event(causing_event_cid)),
        }
    }
}

/// A query with metadata for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryEnvelope<Q> {
    /// Unique identifier for this query instance
    pub id: QueryId,
    /// The actual query
    pub query: Q,
    /// Who issued this query
    pub issued_by: String,
    /// Correlation ID - for new queries, this is self-reference
    pub correlation_id: CorrelationId,
    /// What caused this query (None for user-initiated queries)
    pub causation_id: Option<CausationId>,
}

impl<Q: Query> QueryEnvelope<Q> {
    /// Create a new query envelope (user-initiated, starts new correlation)
    pub fn new(query: Q, issued_by: String) -> Self {
        let id = QueryId::new();
        // New correlation chain - self-reference
        let correlation_id = CorrelationId::from_query(id);

        Self {
            id,
            query,
            issued_by,
            correlation_id,
            causation_id: None, // User-initiated has no causation
        }
    }

    /// Create a query caused by a command (continues correlation)
    pub fn from_command(
        query: Q,
        issued_by: String,
        causing_command_id: CommandId,
        correlation: CorrelationId,
    ) -> Self {
        let id = QueryId::new();

        Self {
            id,
            query,
            issued_by,
            correlation_id: correlation, // Continue existing correlation
            causation_id: Some(CausationId::from_command(causing_command_id)),
        }
    }

    /// Create a query caused by another query (continues correlation)
    pub fn from_query(
        query: Q,
        issued_by: String,
        causing_query_id: QueryId,
        correlation: CorrelationId,
    ) -> Self {
        let id = QueryId::new();

        Self {
            id,
            query,
            issued_by,
            correlation_id: correlation, // Continue existing correlation
            causation_id: Some(CausationId::from_query(causing_query_id)),
        }
    }

    /// Create a query caused by an event (continues correlation)
    pub fn from_event(
        query: Q,
        issued_by: String,
        causing_event_cid: Cid,
        correlation: CorrelationId,
    ) -> Self {
        let id = QueryId::new();

        Self {
            id,
            query,
            issued_by,
            correlation_id: correlation, // Continue existing correlation
            causation_id: Some(CausationId::from_event(causing_event_cid)),
        }
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
/// Handlers return only acknowledgments. Results are published to event streams.
pub trait QueryHandler<Q: Query> {
    /// Handle the query and return acknowledgment
    fn handle(&self, envelope: QueryEnvelope<Q>) -> QueryAcknowledgment;
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

    /// Test command for testing
    #[derive(Debug, Clone)]
    struct TestCommand {
        name: String,
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
        filter: String,
    }

    impl Query for TestQuery {}

    /// Test the flow of command creation and correlation
    ///
    /// ```mermaid
    /// graph LR
    ///     A[User Action] -->|Creates| B[Command]
    ///     B -->|Self-reference| C[CorrelationId]
    ///     B -->|No causation| D[CausationId: None]
    /// ```
    #[test]
    fn test_command_envelope_new() {
        let command = TestCommand {
            name: "test".to_string(),
            aggregate_id: Some(EntityId::new()),
        };

        let envelope = CommandEnvelope::new(command.clone(), "user123".to_string());

        // Verify basic properties
        assert_eq!(envelope.issued_by, "user123");
        assert!(envelope.causation_id.is_none());

        // Verify correlation is self-reference
        match &envelope.correlation_id.0 {
            IdType::Uuid(uuid) => assert_eq!(uuid, envelope.id.as_uuid()),
            _ => panic!("Expected UUID correlation for command"),
        }
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
        let causing_command_id = CommandId::new();
        let correlation = CorrelationId::from_command(causing_command_id);

        let command = TestCommand {
            name: "caused".to_string(),
            aggregate_id: None,
        };

        let envelope = CommandEnvelope::from_command(
            command,
            "system".to_string(),
            causing_command_id,
            correlation.clone(),
        );

        // Verify causation
        assert!(envelope.causation_id.is_some());
        match &envelope.causation_id.unwrap().0 {
            IdType::Uuid(uuid) => assert_eq!(uuid, causing_command_id.as_uuid()),
            _ => panic!("Expected UUID causation"),
        }

        // Verify correlation is preserved
        assert_eq!(envelope.correlation_id, correlation);
    }

    /// Test query envelope creation
    ///
    /// ```mermaid
    /// graph LR
    ///     A[User Query] -->|Creates| B[QueryEnvelope]
    ///     B -->|Self-reference| C[CorrelationId]
    ///     B -->|No causation| D[CausationId: None]
    /// ```
    #[test]
    fn test_query_envelope_new() {
        let query = TestQuery {
            filter: "active".to_string(),
        };

        let envelope = QueryEnvelope::new(query, "user456".to_string());

        // Verify basic properties
        assert_eq!(envelope.issued_by, "user456");
        assert!(envelope.causation_id.is_none());

        // Verify correlation is self-reference
        match &envelope.correlation_id.0 {
            IdType::Uuid(uuid) => assert_eq!(uuid, envelope.id.as_uuid()),
            _ => panic!("Expected UUID correlation for query"),
        }
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
        // Create a mock CID for the event
        let event_cid = Cid::default();
        let correlation = CorrelationId::from_event(event_cid);

        let query = TestQuery {
            filter: "by-event".to_string(),
        };

        let envelope = QueryEnvelope::from_event(
            query,
            "event-handler".to_string(),
            event_cid,
            correlation.clone(),
        );

        // Verify causation
        assert!(envelope.causation_id.is_some());
        match &envelope.causation_id.unwrap().0 {
            IdType::Cid(cid) => assert_eq!(cid, &event_cid),
            _ => panic!("Expected CID causation"),
        }

        // Verify correlation is preserved
        assert_eq!(envelope.correlation_id, correlation);
    }

    /// Test ID type conversions and display
    #[test]
    fn test_id_type_display() {
        let uuid = Uuid::new_v4();
        let uuid_id = IdType::Uuid(uuid);
        assert_eq!(format!("{}", uuid_id), format!("{}", uuid));

        let cid = Cid::default();
        let cid_id = IdType::Cid(cid);
        assert_eq!(format!("{}", cid_id), format!("{}", cid));
    }

    /// Test correlation ID display formats
    #[test]
    fn test_correlation_id_display() {
        let command_id = CommandId::new();
        let correlation = CorrelationId::from_command(command_id);
        let display = format!("{}", correlation);
        assert!(display.starts_with("correlation:"));
        assert!(display.contains(&command_id.as_uuid().to_string()));
    }

    /// Test causation ID display formats
    #[test]
    fn test_causation_id_display() {
        let query_id = QueryId::new();
        let causation = CausationId::from_query(query_id);
        let display = format!("{}", causation);
        assert!(display.starts_with("causation:"));
        assert!(display.contains(&query_id.as_uuid().to_string()));
    }

    /// Test command acknowledgment creation
    #[test]
    fn test_command_acknowledgment() {
        let command_id = CommandId::new();
        let correlation_id = CorrelationId::from_command(command_id);

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
        let correlation_id = CorrelationId::from_command(CommandId::new());
        let causation_id = CausationId::from_query(QueryId::new());

        // Test correlation filter
        let sub1 = EventStreamSubscription::for_correlation(
            "test-stream".to_string(),
            correlation_id.clone(),
        );
        assert_eq!(sub1.stream_name, "test-stream");
        assert_eq!(sub1.correlation_filter, Some(correlation_id));
        assert!(sub1.causation_filter.is_none());

        // Test causation filter
        let sub2 = EventStreamSubscription::for_causation(
            "test-stream".to_string(),
            causation_id.clone(),
        );
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
                correlation_id: envelope.correlation_id,
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
            name: "test".to_string(),
            aggregate_id: None,
        };

        let envelope = CommandEnvelope::new(command, "user".to_string());
        let ack = handler.handle(envelope.clone());

        assert_eq!(ack.command_id, envelope.id);
        assert_eq!(ack.status, CommandStatus::Accepted);
        assert_eq!(*handler.accepted_count.borrow(), 1);
    }
}
