// Copyright 2025 Cowboy AI, LLC.

//! Simplified Schema Export Tool
//! 
//! Extracts JSON schemas from core event payload structs in cim-domain
//! and outputs them as standalone JSON files.

use cim_domain::events::*;
use cim_domain::domain_events::*;
use cim_domain::cqrs::*;
use schemars::{JsonSchema, schema_for};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Generate schema for a type
fn generate_schema<T: JsonSchema>() -> serde_json::Value {
    let schema = schema_for!(T);
    serde_json::to_value(schema).unwrap()
}

/// Export core event schemas
fn export_core_schemas(output_dir: &Path) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let mut schemas = HashMap::new();

    // Create output directory
    fs::create_dir_all(output_dir)?;

    // Core event infrastructure
    println!("Generating schema for: PropagationScope");
    let schema = generate_schema::<PropagationScope>();
    schemas.insert("PropagationScope".to_string(), schema.clone());
    fs::write(output_dir.join("PropagationScope.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: EventMetadata");
    let schema = generate_schema::<EventMetadata>();
    schemas.insert("EventMetadata".to_string(), schema.clone());
    fs::write(output_dir.join("EventMetadata.json"), serde_json::to_string_pretty(&schema)?)?;

    // Domain events
    println!("Generating schema for: DomainEventEnum");
    let schema = generate_schema::<DomainEventEnum>();
    schemas.insert("DomainEventEnum".to_string(), schema.clone());
    fs::write(output_dir.join("DomainEventEnum.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowStarted");
    let schema = generate_schema::<WorkflowStarted>();
    schemas.insert("WorkflowStarted".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowStarted.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowCompleted");
    let schema = generate_schema::<WorkflowCompleted>();
    schemas.insert("WorkflowCompleted".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowCompleted.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowTransitionExecuted");
    let schema = generate_schema::<WorkflowTransitionExecuted>();
    schemas.insert("WorkflowTransitionExecuted".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowTransitionExecuted.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowSuspended");
    let schema = generate_schema::<WorkflowSuspended>();
    schemas.insert("WorkflowSuspended".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowSuspended.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowResumed");
    let schema = generate_schema::<WorkflowResumed>();
    schemas.insert("WorkflowResumed".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowResumed.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowCancelled");
    let schema = generate_schema::<WorkflowCancelled>();
    schemas.insert("WorkflowCancelled".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowCancelled.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowFailed");
    let schema = generate_schema::<WorkflowFailed>();
    schemas.insert("WorkflowFailed".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowFailed.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: WorkflowTransitioned");
    let schema = generate_schema::<WorkflowTransitioned>();
    schemas.insert("WorkflowTransitioned".to_string(), schema.clone());
    fs::write(output_dir.join("WorkflowTransitioned.json"), serde_json::to_string_pretty(&schema)?)?;

    // CQRS types
    println!("Generating schema for: CommandStatus");
    let schema = generate_schema::<CommandStatus>();
    schemas.insert("CommandStatus".to_string(), schema.clone());
    fs::write(output_dir.join("CommandStatus.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: QueryStatus");
    let schema = generate_schema::<QueryStatus>();
    schemas.insert("QueryStatus".to_string(), schema.clone());
    fs::write(output_dir.join("QueryStatus.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: CommandAcknowledgment");
    let schema = generate_schema::<CommandAcknowledgment>();
    schemas.insert("CommandAcknowledgment".to_string(), schema.clone());
    fs::write(output_dir.join("CommandAcknowledgment.json"), serde_json::to_string_pretty(&schema)?)?;

    println!("Generating schema for: QueryResponse");
    let schema = generate_schema::<QueryResponse>();
    schemas.insert("QueryResponse".to_string(), schema.clone());
    fs::write(output_dir.join("QueryResponse.json"), serde_json::to_string_pretty(&schema)?)?;

    // Create a schema index
    let index = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://schemas.cim-domain.ai/index.json",
        "title": "CIM Domain Schema Index",
        "description": "Index of all CIM Domain event payload schemas",
        "version": "0.5.0",
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
        "description": "Combined collection of all CIM Domain event payload schemas",
        "version": "0.5.0",
        "schemas": schemas
    });

    fs::write(output_dir.join("all-schemas.json"), serde_json::to_string_pretty(&all_schemas)?)?;

    Ok(schemas)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("schemas");
    
    println!("Exporting CIM Domain schemas to: {}", output_dir.display());
    
    let schemas = export_core_schemas(output_dir)?;
    
    println!("\n✅ Successfully exported {} schemas:", schemas.len());
    for schema_name in schemas.keys() {
        println!("  - {}", schema_name);
    }
    
    println!("\n📄 Files created:");
    println!("  - schemas/index.json (schema index)");
    println!("  - schemas/all-schemas.json (combined schemas)");
    for schema_name in schemas.keys() {
        println!("  - schemas/{}.json", schema_name);
    }
    
    println!("\n🎯 Schemas are now available as standalone JSON without requiring cim-domain dependency");
    println!("💡 These schemas can be used for:");
    println!("  - Event payload validation");
    println!("  - API documentation generation");
    println!("  - Code generation in other languages");
    println!("  - Integration testing");
    
    Ok(())
}