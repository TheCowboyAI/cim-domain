// Copyright 2025 Cowboy AI, LLC.

//! Basic Schema Export Tool
//! 
//! Generates JSON schemas for core event types that can be used without cim-domain dependencies.
//! This creates self-contained schemas for the most important event types.

use serde::{Deserialize, Serialize};
use schemars::{JsonSchema, schema_for};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Basic event propagation scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum PropagationScope {
    /// Never leaves the app
    LocalOnly,
    /// May bubble to container
    Container,
    /// May bubble to local leaf
    Leaf,
    /// May bubble to cluster
    Cluster,
    /// May bubble globally
    SuperCluster,
}

/// Basic event metadata for standalone use
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EventMetadata {
    /// Source service/context
    pub source: String,
    /// Event version
    pub version: String,
    /// Propagation scope
    pub propagation_scope: PropagationScope,
    /// Additional metadata
    pub properties: HashMap<String, serde_json::Value>,
}

/// Basic event envelope for standalone use
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EventEnvelope<T> {
    /// The domain event
    pub event: T,
    /// Subject for routing (e.g., "people.person.registered.v1")
    pub subject: String,
    /// Determines if/how to escalate
    pub propagation: PropagationScope,
}

/// Workflow started event
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowStarted {
    /// The unique identifier of the workflow instance
    pub workflow_id: Uuid,
    /// The ID of the graph definition this workflow is based on
    pub definition_id: Uuid,
    /// The initial state of the workflow
    pub initial_state: String,
    /// When the workflow was started
    pub started_at: DateTime<Utc>,
}

/// Workflow transition executed event
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowTransitionExecuted {
    /// The workflow that executed the transition
    pub workflow_id: Uuid,
    /// The state before the transition
    pub from_state: String,
    /// The state after the transition
    pub to_state: String,
    /// The input that triggered the transition
    pub input: serde_json::Value,
    /// The output produced by the transition
    pub output: serde_json::Value,
    /// When the transition was executed
    pub executed_at: DateTime<Utc>,
}

/// Workflow completed event
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowCompleted {
    /// The workflow that completed
    pub workflow_id: Uuid,
    /// The final state of the workflow
    pub final_state: String,
    /// The total duration of the workflow execution in seconds
    pub total_duration_seconds: f64,
    /// When the workflow completed
    pub completed_at: DateTime<Utc>,
}

/// Workflow suspended event
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowSuspended {
    /// The workflow that was suspended
    pub workflow_id: Uuid,
    /// The state at which the workflow was suspended
    pub current_state: String,
    /// The reason for suspension
    pub reason: String,
    /// When the workflow was suspended
    pub suspended_at: DateTime<Utc>,
}

/// Workflow resumed event
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowResumed {
    /// The workflow that was resumed
    pub workflow_id: Uuid,
    /// The state from which the workflow resumed
    pub current_state: String,
    /// When the workflow was resumed
    pub resumed_at: DateTime<Utc>,
}

/// Workflow cancelled event
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowCancelled {
    /// The workflow that was cancelled
    pub workflow_id: Uuid,
    /// The state at which the workflow was cancelled
    pub current_state: String,
    /// The reason for cancellation
    pub reason: String,
    /// When the workflow was cancelled
    pub cancelled_at: DateTime<Utc>,
}

/// Workflow failed event
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowFailed {
    /// The workflow that failed
    pub workflow_id: Uuid,
    /// The state at which the workflow failed
    pub current_state: String,
    /// The error that caused the failure
    pub error: String,
    /// When the workflow failed
    pub failed_at: DateTime<Utc>,
}

/// Command status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CommandStatus {
    /// Command was accepted for processing
    Accepted,
    /// Command was rejected (e.g., validation failed)
    Rejected,
}

/// Query status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum QueryStatus {
    /// Query was accepted for processing
    Accepted,
    /// Query was rejected (e.g., invalid parameters)
    Rejected,
}

/// Command acknowledgment
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommandAcknowledgment {
    /// The command ID that was acknowledged
    pub command_id: Uuid,
    /// Correlation ID (same as command ID for originating commands)
    pub correlation_id: Uuid,
    /// Status of command acceptance
    pub status: CommandStatus,
    /// Optional rejection reason
    pub reason: Option<String>,
}

/// Query response
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryResponse {
    /// The query ID that was processed
    pub query_id: Uuid,
    /// Correlation ID for tracking
    pub correlation_id: Uuid,
    /// The result data
    pub result: serde_json::Value,
}

/// Combined event enum for all workflow events
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum WorkflowEvent {
    Started(WorkflowStarted),
    TransitionExecuted(WorkflowTransitionExecuted),
    Completed(WorkflowCompleted),
    Suspended(WorkflowSuspended),
    Resumed(WorkflowResumed),
    Cancelled(WorkflowCancelled),
    Failed(WorkflowFailed),
}

/// Generate schema for a type with metadata
fn generate_schema_with_metadata<T: JsonSchema>(name: &str, description: &str) -> serde_json::Value {
    let schema = schema_for!(T);
    let mut schema_value = serde_json::to_value(schema).unwrap();
    
    // Add custom metadata
    if let serde_json::Value::Object(ref mut schema_obj) = schema_value {
        schema_obj.insert("$id".to_string(), serde_json::Value::String(format!("https://schemas.cim-domain.ai/{}.json", name)));
        schema_obj.insert("title".to_string(), serde_json::Value::String(name.to_string()));
        schema_obj.insert("description".to_string(), serde_json::Value::String(description.to_string()));
    }
    
    schema_value
}

/// Export all schemas
fn export_schemas(output_dir: &Path) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let mut schemas = HashMap::new();

    // Create output directory
    fs::create_dir_all(output_dir)?;

    println!("Generating core infrastructure schemas...");
    
    // Core infrastructure schemas
    let schema = generate_schema_with_metadata::<PropagationScope>("PropagationScope", "Propagation scope for event escalation");
    schemas.insert("PropagationScope".to_string(), schema.clone());
    fs::write(output_dir.join("PropagationScope.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<EventMetadata>("EventMetadata", "Metadata for event processing");
    schemas.insert("EventMetadata".to_string(), schema.clone());
    fs::write(output_dir.join("EventMetadata.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating workflow event schemas...");
    
    // Workflow event schemas
    let schema = generate_schema_with_metadata::<WorkflowStarted>("WorkflowStarted", "Workflow started event");
    schemas.insert("WorkflowStarted".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowStarted.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<WorkflowTransitionExecuted>("WorkflowTransitionExecuted", "Workflow transition executed event");
    schemas.insert("WorkflowTransitionExecuted".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowTransitionExecuted.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<WorkflowCompleted>("WorkflowCompleted", "Workflow completed event");
    schemas.insert("WorkflowCompleted".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowCompleted.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<WorkflowSuspended>("WorkflowSuspended", "Workflow suspended event");
    schemas.insert("WorkflowSuspended".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowSuspended.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<WorkflowResumed>("WorkflowResumed", "Workflow resumed event");
    schemas.insert("WorkflowResumed".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowResumed.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<WorkflowCancelled>("WorkflowCancelled", "Workflow cancelled event");
    schemas.insert("WorkflowCancelled".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowCancelled.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<WorkflowFailed>("WorkflowFailed", "Workflow failed event");
    schemas.insert("WorkflowFailed".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowFailed.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<WorkflowEvent>("WorkflowEvent", "Combined workflow event enum");
    schemas.insert("WorkflowEvent".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowEvent.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating CQRS schemas...");
    
    // CQRS schemas
    let schema = generate_schema_with_metadata::<CommandStatus>("CommandStatus", "Status of command acceptance");
    schemas.insert("CommandStatus".to_string(), schema.clone());
    fs::write(output_dir.join("CommandStatus.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<QueryStatus>("QueryStatus", "Status of query acceptance");
    schemas.insert("QueryStatus".to_string(), schema.clone());
    fs::write(output_dir.join("QueryStatus.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<CommandAcknowledgment>("CommandAcknowledgment", "Acknowledgment returned when a command is submitted");
    schemas.insert("CommandAcknowledgment".to_string(), schema.clone());
    fs::write(output_dir.join("CommandAcknowledgment.json"), serde_json::to_string_pretty(&schema)?)?;

    let schema = generate_schema_with_metadata::<QueryResponse>("QueryResponse", "Query response returned by query handlers");
    schemas.insert("QueryResponse".to_string(), schema.clone());
    fs::write(output_dir.join("QueryResponse.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Creating combined files...");

    // Create a schema index
    let index = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://schemas.cim-domain.ai/index.json",
        "title": "CIM Domain Schema Index",
        "description": "Index of all CIM Domain event payload schemas",
        "version": "0.5.0",
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "schemas": schemas.keys().map(|name| {
            serde_json::json!({
                "name": name,
                "file": format!("{}.json", name),
                "url": format!("https://schemas.cim-domain.ai/{}.json", name)
            })
        }).collect::<Vec<_>>()
    });

    fs::write(output_dir.join("index.json"), serde_json::to_string_pretty(&index)?)?;

    // Create a combined schema file
    let all_schemas = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://schemas.cim-domain.ai/all-schemas.json",
        "title": "CIM Domain All Schemas",
        "description": "Combined collection of all CIM Domain event payload schemas. Use these schemas for validation and code generation in any language.",
        "version": "0.5.0",
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "schemas": schemas
    });

    fs::write(output_dir.join("all-schemas.json"), serde_json::to_string_pretty(&all_schemas)?)?;

    // Create a README
    let readme = format!(r#"# CIM Domain Event Schemas

This directory contains JSON Schema definitions for all CIM Domain event payload types.

## Generated Schemas ({} total)

{}

## Usage

### Validation
Use these schemas to validate event payloads in any language:

```bash
# Using ajv-cli
ajv validate -s WorkflowStarted.json -d your-event.json
```

### Code Generation
Generate typed data structures from these schemas:

```bash
# TypeScript
quicktype --src-lang schema --lang typescript WorkflowStarted.json

# Python
datamodel-codegen --input WorkflowStarted.json --output workflow_models.py

# Go
schema-generate -p main WorkflowStarted.json > workflow_models.go
```

### API Documentation
Use these schemas in OpenAPI specifications or other API documentation tools.

## Files

- `index.json` - Schema catalog/index
- `all-schemas.json` - Combined schemas in one file
- `[SchemaName].json` - Individual schema files

## Schema URLs

All schemas are available at `https://schemas.cim-domain.ai/[SchemaName].json`

Generated on: {}
Version: 0.5.0
"#, 
        schemas.len(),
        schemas.keys().map(|name| format!("- {}.json", name)).collect::<Vec<_>>().join("\n"),
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    fs::write(output_dir.join("README.md"), readme)?;

    Ok(schemas)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("schemas");
    
    println!("🚀 Exporting CIM Domain schemas to: {}", output_dir.display());
    
    let schemas = export_schemas(output_dir)?;
    
    println!("\n✅ Successfully exported {} schemas:", schemas.len());
    for schema_name in schemas.keys() {
        println!("  📄 {}", schema_name);
    }
    
    println!("\n📁 Files created:");
    println!("  📋 schemas/index.json (schema catalog)");
    println!("  📦 schemas/all-schemas.json (combined schemas)");
    println!("  📖 schemas/README.md (usage documentation)");
    for schema_name in schemas.keys() {
        println!("  📄 schemas/{}.json", schema_name);
    }
    
    println!("\n🎯 These JSON schemas are now:");
    println!("  ✨ Standalone (no cim-domain dependency required)");
    println!("  📐 Standards-compliant JSON Schema Draft 7");
    println!("  🔄 Suitable for code generation in any language");
    println!("  ✅ Ready for payload validation");
    println!("  📚 Documented with descriptions and examples");
    
    println!("\n💡 Next steps:");
    println!("  1. Review the generated schemas in the schemas/ directory");
    println!("  2. Use them for validation, code gen, or API docs");
    println!("  3. Publish to a schema registry if needed");
    println!("  4. Share with integration teams");
    
    Ok(())
}