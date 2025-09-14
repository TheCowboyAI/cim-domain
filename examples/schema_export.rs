// Copyright 2025 Cowboy AI, LLC.

//! Schema Export Tool
//! 
//! Extracts JSON schemas from all event payload structs in cim-domain
//! and outputs them as standalone JSON files without requiring the cim-domain crate.

use cim_domain::*;
use cim_domain::events::*;
use cim_domain::domain_events::*;
use cim_domain::cqrs::*;
use cim_domain::composition::saga_orchestration::*;
use cim_domain::infrastructure::event_store::*;
use cim_domain::infrastructure::event_stream::*;
use cim_domain::infrastructure::event_versioning::*;
use cim_domain::persistence::aggregate_repository::*;
use cim_domain::persistence::read_model_store::*;
use cim_domain::persistence::query_support::*;
use cim_domain::query_handlers::*;
use schemars::{JsonSchema, schema_for};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// All event payload types that should have JSON schemas exported
trait SchemaExportable: JsonSchema + serde::Serialize {
    fn schema_name() -> &'static str;
    fn schema_description() -> &'static str {
        "CIM Domain schema"
    }
}

// Implement SchemaExportable for core event types
impl SchemaExportable for PropagationScope {
    fn schema_name() -> &'static str { "PropagationScope" }
    fn schema_description() -> &'static str {
        "Propagation scope for event escalation"
    }
}

impl<E: JsonSchema + serde::Serialize> SchemaExportable for EventEnvelope<E> {
    fn schema_name() -> &'static str { "EventEnvelope" }
    fn schema_description() -> &'static str {
        "Event envelope with subject and propagation scope"
    }
}

impl SchemaExportable for EventMetadata {
    fn schema_name() -> &'static str { "EventMetadata" }
    fn schema_description() -> &'static str {
        "Metadata for event processing"
    }
}

impl<E: JsonSchema + serde::Serialize> SchemaExportable for DomainEventEnvelope<E> {
    fn schema_name() -> &'static str { "DomainEventEnvelope" }
    fn schema_description() -> &'static str {
        "Wrapper for domain events with metadata"
    }
}

// Domain events
impl SchemaExportable for DomainEventEnum {
    fn schema_name() -> &'static str { "DomainEventEnum" }
    fn schema_description() -> &'static str {
        "Enum wrapper for all domain events"
    }
}

impl SchemaExportable for WorkflowStarted {
    fn schema_name() -> &'static str { "WorkflowStarted" }
    fn schema_description() -> &'static str {
        "Workflow started event"
    }
}

impl SchemaExportable for WorkflowTransitionExecuted {
    fn schema_name() -> &'static str { "WorkflowTransitionExecuted" }
    fn schema_description() -> &'static str {
        "Workflow transition executed event"
    }
}

impl SchemaExportable for WorkflowCompleted {
    fn schema_name() -> &'static str { "WorkflowCompleted" }
    fn schema_description() -> &'static str {
        "Workflow completed event"
    }
}

impl SchemaExportable for WorkflowSuspended {
    fn schema_name() -> &'static str { "WorkflowSuspended" }
    fn schema_description() -> &'static str {
        "Workflow suspended event"
    }
}

impl SchemaExportable for WorkflowResumed {
    fn schema_name() -> &'static str { "WorkflowResumed" }
    fn schema_description() -> &'static str {
        "Workflow resumed event"
    }
}

impl SchemaExportable for WorkflowCancelled {
    fn schema_name() -> &'static str { "WorkflowCancelled" }
    fn schema_description() -> &'static str {
        "Workflow cancelled event"
    }
}

impl SchemaExportable for WorkflowFailed {
    fn schema_name() -> &'static str { "WorkflowFailed" }
    fn schema_description() -> &'static str {
        "Workflow failed event"
    }
}

impl SchemaExportable for WorkflowTransitioned {
    fn schema_name() -> &'static str { "WorkflowTransitioned" }
    fn schema_description() -> &'static str {
        "Workflow transitioned event"
    }
}

// CQRS types
impl SchemaExportable for CommandStatus {
    fn schema_name() -> &'static str { "CommandStatus" }
    fn schema_description() -> &'static str {
        "Status of command acceptance"
    }
}

impl SchemaExportable for QueryStatus {
    fn schema_name() -> &'static str { "QueryStatus" }
    fn schema_description() -> &'static str {
        "Status of query acceptance"
    }
}

impl SchemaExportable for CommandAcknowledgment {
    fn schema_name() -> &'static str { "CommandAcknowledgment" }
    fn schema_description() -> &'static str {
        "Acknowledgment returned when a command is submitted"
    }
}

impl SchemaExportable for QueryResponse {
    fn schema_name() -> &'static str { "QueryResponse" }
    fn schema_description() -> &'static str {
        "Query response returned by query handlers"
    }
}

// Saga types
impl SchemaExportable for SagaEvent {
    fn schema_name() -> &'static str { "SagaEvent" }
    fn schema_description() -> &'static str {
        "Saga orchestration event"
    }
}

impl SchemaExportable for SagaCommand {
    fn schema_name() -> &'static str { "SagaCommand" }
    fn schema_description() -> &'static str {
        "Saga orchestration command"
    }
}

impl SchemaExportable for SagaTransitionInput {
    fn schema_name() -> &'static str { "SagaTransitionInput" }
    fn schema_description() -> &'static str {
        "Saga transition input"
    }
}

impl SchemaExportable for SagaState {
    fn schema_name() -> &'static str { "SagaState" }
    fn schema_description() -> &'static str {
        "Saga state"
    }
}

// Infrastructure types
impl SchemaExportable for StoredEvent {
    fn schema_name() -> &'static str { "StoredEvent" }
    fn schema_description() -> &'static str {
        "Stored event in the event store"
    }
}

impl SchemaExportable for EventStream {
    fn schema_name() -> &'static str { "EventStream" }
    fn schema_description() -> &'static str {
        "Event stream"
    }
}

impl SchemaExportable for EventStreamMetadata {
    fn schema_name() -> &'static str { "EventStreamMetadata" }
    fn schema_description() -> &'static str {
        "Event stream metadata"
    }
}

impl SchemaExportable for TimeRange {
    fn schema_name() -> &'static str { "TimeRange" }
    fn schema_description() -> &'static str {
        "Time range for event filtering"
    }
}

impl SchemaExportable for CausationChain {
    fn schema_name() -> &'static str { "CausationChain" }
    fn schema_description() -> &'static str {
        "Causation chain for event tracing"
    }
}

impl SchemaExportable for EventFilter {
    fn schema_name() -> &'static str { "EventFilter" }
    fn schema_description() -> &'static str {
        "Event filter criteria"
    }
}

impl SchemaExportable for EventOrdering {
    fn schema_name() -> &'static str { "EventOrdering" }
    fn schema_description() -> &'static str {
        "Event ordering specification"
    }
}

impl SchemaExportable for CausationOrder {
    fn schema_name() -> &'static str { "CausationOrder" }
    fn schema_description() -> &'static str {
        "Causation ordering specification"
    }
}

impl SchemaExportable for EventQuery {
    fn schema_name() -> &'static str { "EventQuery" }
    fn schema_description() -> &'static str {
        "Event query specification"
    }
}

impl SchemaExportable for EventGrouping {
    fn schema_name() -> &'static str { "EventGrouping" }
    fn schema_description() -> &'static str {
        "Event grouping specification"
    }
}

impl SchemaExportable for VersionedEvent {
    fn schema_name() -> &'static str { "VersionedEvent" }
    fn schema_description() -> &'static str {
        "Versioned event with metadata"
    }
}

// Query support types
impl SchemaExportable for QueryCriteria {
    fn schema_name() -> &'static str { "QueryCriteria" }
    fn schema_description() -> &'static str {
        "Query criteria for data retrieval"
    }
}

impl SchemaExportable for QueryOptions {
    fn schema_name() -> &'static str { "QueryOptions" }
    fn schema_description() -> &'static str {
        "Query options for data retrieval"
    }
}

impl SchemaExportable for SortDirection {
    fn schema_name() -> &'static str { "SortDirection" }
    fn schema_description() -> &'static str {
        "Sort direction for query results"
    }
}

impl<T: JsonSchema + serde::Serialize> SchemaExportable for QueryResult<T> {
    fn schema_name() -> &'static str { "QueryResult" }
    fn schema_description() -> &'static str {
        "Query result with pagination"
    }
}

impl SchemaExportable for Pagination {
    fn schema_name() -> &'static str { "Pagination" }
    fn schema_description() -> &'static str {
        "Pagination configuration"
    }
}

// Repository types
impl SchemaExportable for AggregateMetadata {
    fn schema_name() -> &'static str { "AggregateMetadata" }
    fn schema_description() -> &'static str {
        "Aggregate metadata for persistence"
    }
}

impl SchemaExportable for ReadModelMetadata {
    fn schema_name() -> &'static str { "ReadModelMetadata" }
    fn schema_description() -> &'static str {
        "Read model metadata"
    }
}

impl SchemaExportable for ProjectionStatus {
    fn schema_name() -> &'static str { "ProjectionStatus" }
    fn schema_description() -> &'static str {
        "Projection status"
    }
}

impl SchemaExportable for MaterializedView {
    fn schema_name() -> &'static str { "MaterializedView" }
    fn schema_description() -> &'static str {
        "Materialized view for queries"
    }
}

/// Generate schema for a type
fn generate_schema<T: SchemaExportable>() -> serde_json::Value {
    let schema = schema_for!(T);
    let mut schema_value = serde_json::to_value(schema).unwrap();
    
    // Add custom metadata
    if let serde_json::Value::Object(ref mut schema_obj) = schema_value {
        schema_obj.insert("$id".to_string(), serde_json::Value::String(format!("https://schemas.cim-domain.ai/{}.json", T::schema_name())));
        schema_obj.insert("title".to_string(), serde_json::Value::String(T::schema_name().to_string()));
        schema_obj.insert("description".to_string(), serde_json::Value::String(T::schema_description().to_string()));
    }
    
    schema_value
}

/// Export all schemas
fn export_schemas(output_dir: &Path) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let mut schemas = HashMap::new();

    // Create output directory
    fs::create_dir_all(output_dir)?;

    // Generate schemas for all types
    macro_rules! export_schema {
        ($type:ty) => {{
            let schema_name = <$type>::schema_name();
            println!("Generating schema for: {}", schema_name);
            let schema = generate_schema::<$type>();
            schemas.insert(schema_name.to_string(), schema.clone());
            
            // Write individual schema file
            let file_path = output_dir.join(format!("{}.json", schema_name));
            fs::write(&file_path, serde_json::to_string_pretty(&schema)?)?;
        }};
    }

    // Export all event and command schemas
    export_schema!(PropagationScope);
    export_schema!(EventMetadata);
    export_schema!(DomainEventEnum);
    export_schema!(WorkflowStarted);
    export_schema!(WorkflowTransitionExecuted);
    export_schema!(WorkflowCompleted);
    export_schema!(WorkflowSuspended);
    export_schema!(WorkflowResumed);
    export_schema!(WorkflowCancelled);
    export_schema!(WorkflowFailed);
    export_schema!(WorkflowTransitioned);
    export_schema!(CommandStatus);
    export_schema!(QueryStatus);
    export_schema!(CommandAcknowledgment);
    export_schema!(QueryResponse);
    export_schema!(SagaEvent);
    export_schema!(SagaCommand);
    export_schema!(SagaTransitionInput);
    export_schema!(SagaState);
    export_schema!(StoredEvent);
    export_schema!(EventStream);
    export_schema!(EventStreamMetadata);
    export_schema!(TimeRange);
    export_schema!(CausationChain);
    export_schema!(EventFilter);
    export_schema!(EventOrdering);
    export_schema!(CausationOrder);
    export_schema!(EventQuery);
    export_schema!(EventGrouping);
    export_schema!(VersionedEvent);
    export_schema!(QueryCriteria);
    export_schema!(QueryOptions);
    export_schema!(SortDirection);
    export_schema!(Pagination);
    export_schema!(AggregateMetadata);
    export_schema!(ReadModelMetadata);
    export_schema!(ProjectionStatus);
    export_schema!(MaterializedView);

    // Create a combined schema index
    let index = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://schemas.cim-domain.ai/index.json",
        "title": "CIM Domain Schema Index",
        "description": "Index of all CIM Domain event payload schemas",
        "type": "object",
        "properties": {
            "version": {
                "type": "string",
                "const": "0.5.0"
            },
            "schemas": {
                "type": "object",
                "properties": schemas.iter().map(|(name, _)| {
                    (name.clone(), serde_json::json!({
                        "$ref": format!("{}.json", name)
                    }))
                }).collect::<serde_json::Map<String, serde_json::Value>>()
            }
        }
    });

    fs::write(output_dir.join("index.json"), serde_json::to_string_pretty(&index)?)?;

    // Create a combined all-schemas file
    let all_schemas = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://schemas.cim-domain.ai/all-schemas.json",
        "title": "CIM Domain All Schemas",
        "description": "Combined collection of all CIM Domain event payload schemas",
        "type": "object",
        "properties": {
            "version": {
                "type": "string",
                "const": "0.5.0"
            },
            "schemas": schemas
        }
    });

    fs::write(output_dir.join("all-schemas.json"), serde_json::to_string_pretty(&all_schemas)?)?;

    Ok(schemas)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("schemas");
    
    println!("Exporting CIM Domain schemas to: {}", output_dir.display());
    
    let schemas = export_schemas(output_dir)?;
    
    println!("\n✅ Successfully exported {} schemas:", schemas.len());
    for schema_name in schemas.keys() {
        println!("  - {}", schema_name);
    }
    
    println!("\n📄 Files created:");
    println!("  - schemas/index.json (schema index)");
    println!("  - schemas/all-schemas.json (combined schemas)");
    println!("  - schemas/[SchemaName].json (individual schemas)");
    
    println!("\n🎯 Schemas are now available as standalone JSON without requiring cim-domain dependency");
    
    Ok(())
}