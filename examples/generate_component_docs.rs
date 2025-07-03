//! Generate component documentation from JSON
//!
//! This example reads the components.json file and generates
//! markdown documentation with proper formatting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
struct ComponentsJson {
    module: String,
    version: String,
    description: String,
    components: HashMap<String, ModuleInfo>,
    statistics: Statistics,
    core_entities: Vec<CoreEntity>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModuleInfo {
    file: String,
    description: String,
    exports: Vec<Export>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Export {
    name: String,
    #[serde(rename = "type")]
    type_kind: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    generic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variants: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Statistics {
    traits: u32,
    enums: u32,
    structs: u32,
    type_aliases: u32,
    core_entities: u32,
    event_types: u32,
    command_types: u32,
    total_public_types: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct CoreEntity {
    name: String,
    description: String,
    events: Vec<String>,
    commands: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the JSON file
    let json_path = Path::new("doc/design/components.json");
    let json_content = fs::read_to_string(json_path)?;
    let components: ComponentsJson = serde_json::from_str(&json_content)?;

    // Generate markdown
    let mut markdown = String::new();

    // Header
    markdown.push_str(&format!("# {components.module} Component Reference\n\n"));
    markdown.push_str(&format!("**Version**: {components.version}\n\n"));
    markdown.push_str(&format!("**Description**: {components.description}\n\n"));

    // Table of Contents
    markdown.push_str("## Table of Contents\n\n");
    markdown.push_str("1. [Component Overview](#component-overview)\n");
    markdown.push_str("2. [Core Entities](#core-entities)\n");
    markdown.push_str("3. [Module Reference](#module-reference)\n");
    markdown.push_str("4. [Statistics](#statistics)\n");
    markdown.push_str("5. [Type Index](#type-index)\n\n");

    // Component Overview with Mermaid diagram
    markdown.push_str("## Component Overview\n\n");
    markdown.push_str("```mermaid\n");
    markdown.push_str("graph LR\n");
    markdown.push_str("    subgraph \"Core DDD\"\n");
    markdown.push_str("        Component\n");
    markdown.push_str("        Entity\n");
    markdown.push_str("        AggregateRoot\n");
    markdown.push_str("    end\n");
    markdown.push_str("    subgraph \"CQRS\"\n");
    markdown.push_str("        Command\n");
    markdown.push_str("        Query\n");
    markdown.push_str("        CommandHandler\n");
    markdown.push_str("        QueryHandler\n");
    markdown.push_str("    end\n");
    markdown.push_str("    subgraph \"Events\"\n");
    markdown.push_str("        DomainEvent\n");
    markdown.push_str("        EventMetadata\n");
    markdown.push_str("    end\n");
    markdown.push_str("    subgraph \"Entities\"\n");
    for entity in &components.core_entities {
        markdown.push_str(&format!("        {entity.name}\n"));
    }
    markdown.push_str("    end\n");
    markdown.push_str("```\n\n");

    // Core Entities
    markdown.push_str("## Core Entities\n\n");
    for entity in &components.core_entities {
        markdown.push_str(&format!("### {entity.name}\n\n"));
        markdown.push_str(&format!("{entity.description}\n\n"));

        markdown.push_str("**Events**:\n");
        for event in &entity.events {
            markdown.push_str(&format!("- `{event}`\n"));
        }

        markdown.push_str("\n**Commands**:\n");
        for command in &entity.commands {
            markdown.push_str(&format!("- `{command}`\n"));
        }
        markdown.push_str("\n");
    }

    // Module Reference
    markdown.push_str("## Module Reference\n\n");

    // Sort modules by name for consistent output
    let mut modules: Vec<_> = components.components.iter().collect();
    modules.sort_by_key(|(name, _)| name.as_str());

    for (module_name, module_info) in modules {
        markdown.push_str(&format!("### {module_name} (`{module_info.file}`)\n\n"));
        markdown.push_str(&format!("{module_info.description}\n\n"));

        // Group exports by type
        let mut traits = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut type_aliases = Vec::new();

        for export in &module_info.exports {
            match export.type_kind.as_str() {
                "trait" => traits.push(export),
                "struct" => structs.push(export),
                "enum" => enums.push(export),
                "type_alias" => type_aliases.push(export),
                _ => {}
            }
        }

        // Output by type
        if !traits.is_empty() {
            markdown.push_str("**Traits**:\n");
            for t in traits {
                markdown.push_str(&format!("- `{t.name}` - {t.description}\n"));
            }
            markdown.push_str("\n");
        }

        if !structs.is_empty() {
            markdown.push_str("**Structs**:\n");
            for s in structs {
                let generic = s.generic.as_ref().map(|g| format!("<{g}>")).unwrap_or_default();
                markdown.push_str(&format!("- `{s.name}{generic}` - {s.description}\n"));
            }
            markdown.push_str("\n");
        }

        if !enums.is_empty() {
            markdown.push_str("**Enums**:\n");
            for e in enums {
                markdown.push_str(&format!("- `{e.name}` - {e.description}"));
                if let Some(variants) = &e.variants {
                    markdown.push_str(&format!(" ({variants.join(", "})")));
                }
                markdown.push_str("\n");
            }
            markdown.push_str("\n");
        }

        if !type_aliases.is_empty() {
            markdown.push_str("**Type Aliases**:\n");
            for ta in type_aliases {
                let generic = ta.generic.as_ref().map(|g| format!("<{g}>")).unwrap_or_default();
                markdown.push_str(&format!("- `{ta.name}{generic}` - {ta.description}\n"));
            }
            markdown.push_str("\n");
        }
    }

    // Statistics
    markdown.push_str("## Statistics\n\n");
    markdown.push_str("| Type | Count |\n");
    markdown.push_str("|------|-------|\n");
    markdown.push_str(&format!("| Traits | {components.statistics.traits} |\n"));
    markdown.push_str(&format!("| Enums | {components.statistics.enums} |\n"));
    markdown.push_str(&format!("| Structs | {components.statistics.structs} |\n"));
    markdown.push_str(&format!("| Type Aliases | {components.statistics.type_aliases} |\n"));
    markdown.push_str(&format!("| Core Entities | {components.statistics.core_entities} |\n"));
    markdown.push_str(&format!("| Event Types | {components.statistics.event_types} |\n"));
    markdown.push_str(&format!("| Command Types | {components.statistics.command_types} |\n"));
    markdown.push_str(&format!("| **Total Public Types** | **{components.statistics.total_public_types}** |\n\n"));

    // Type Index
    markdown.push_str("## Type Index\n\n");
    markdown.push_str("All public types in alphabetical order:\n\n");

    // Collect all types
    let mut all_types = Vec::new();
    for (module_name, module_info) in &components.components {
        for export in &module_info.exports {
            all_types.push((export.name.clone(), export.type_kind.clone(), module_name.clone()));
        }
    }
    all_types.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, type_kind, module) in all_types {
        markdown.push_str(&format!("- `{name}` ({type_kind}) - [{module}](#{module})\n"));
    }

    // Write the markdown file
    let output_path = Path::new("doc/design/components-generated.md");
    fs::write(output_path, markdown)?;

    println!("Generated markdown documentation at: {output_path.display(}"));

    Ok(())
}
