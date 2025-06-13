//! Generate domain model graph visualization
//!
//! This binary analyzes the cim-domain model and generates graph visualizations
//! in both Mermaid and GraphViz DOT formats.

use cim_domain::domain_graph::DomainGraph;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Analyzing domain model...");

    // Analyze the current domain model
    let graph = DomainGraph::analyze_current_model();

    // Create output directory
    let output_dir = Path::new("domain-graphs");
    fs::create_dir_all(output_dir)?;

    // Generate Mermaid diagram
    let mermaid = graph.to_mermaid();
    let mermaid_path = output_dir.join("domain-model.mmd");
    fs::write(&mermaid_path, mermaid)?;
    println!("Generated Mermaid diagram: {}", mermaid_path.display());

    // Generate GraphViz DOT
    let dot = graph.to_dot();
    let dot_path = output_dir.join("domain-model.dot");
    fs::write(&dot_path, dot)?;
    println!("Generated DOT diagram: {}", dot_path.display());

    // Find unused elements
    let unused = graph.find_unused_elements();
    if !unused.is_empty() {
        println!("\nWarning: Found {} unused elements:", unused.len());
        for node in unused {
            println!("  - {} ({})", node.name, node.module);
        }
    }

    // Find missing elements
    let missing = graph.find_missing_elements();
    if !missing.is_empty() {
        println!("\nWarning: Found {} missing elements referenced in relationships:", missing.len());
        for name in missing {
            println!("  - {}", name);
        }
    }

    // Generate analysis report
    let report = generate_analysis_report(&graph);
    let report_path = output_dir.join("domain-analysis.md");
    fs::write(&report_path, report)?;
    println!("\nGenerated analysis report: {}", report_path.display());

    println!("\nDomain model analysis complete!");
    println!("\nTo view the Mermaid diagram, paste the contents of {} into:", mermaid_path.display());
    println!("  https://mermaid.live/");
    println!("\nTo generate a PNG from the DOT file, run:");
    println!("  dot -Tpng {} -o domain-model.png", dot_path.display());

    Ok(())
}

fn generate_analysis_report(graph: &DomainGraph) -> String {
    let mut report = String::new();

    report.push_str("# Domain Model Analysis Report\n\n");
    report.push_str("## Summary\n\n");

    // Count elements by type
    let mut type_counts = std::collections::HashMap::new();
    for node in graph.nodes.values() {
        *type_counts.entry(&node.element_type).or_insert(0) += 1;
    }

    report.push_str("### Element Counts\n\n");
    report.push_str("| Type | Count |\n");
    report.push_str("|------|-------|\n");
    for (element_type, count) in type_counts {
        report.push_str(&format!("| {:?} | {} |\n", element_type, count));
    }

    report.push_str("\n### Relationship Summary\n\n");
    report.push_str(&format!("Total relationships: {}\n\n", graph.edges.len()));

    // Group relationships by type
    let mut rel_counts = std::collections::HashMap::new();
    for edge in &graph.edges {
        *rel_counts.entry(&edge.relationship).or_insert(0) += 1;
    }

    report.push_str("| Relationship Type | Count |\n");
    report.push_str("|-------------------|-------|\n");
    for (rel_type, count) in rel_counts {
        report.push_str(&format!("| {:?} | {} |\n", rel_type, count));
    }

    report.push_str("\n## Detailed Element List\n\n");

    // Group nodes by module
    let mut modules = std::collections::HashMap::new();
    for node in graph.nodes.values() {
        modules.entry(&node.module).or_insert_with(Vec::new).push(node);
    }

    for (module, nodes) in modules {
        report.push_str(&format!("### Module: {}\n\n", module));

        for node in nodes {
            report.push_str(&format!("#### {} ({:?})\n\n", node.name, node.element_type));

            if !node.fields.is_empty() {
                report.push_str("**Fields:**\n");
                for field in &node.fields {
                    let opt = if field.is_optional { "?" } else { "" };
                    let coll = if field.is_collection { "[]" } else { "" };
                    report.push_str(&format!("- {}: {}{}{}\n", field.name, field.field_type, coll, opt));
                }
                report.push_str("\n");
            }

            if !node.methods.is_empty() {
                report.push_str("**Methods:**\n");
                for method in &node.methods {
                    report.push_str(&format!("- {}\n", method));
                }
                report.push_str("\n");
            }

            if !node.traits.is_empty() {
                report.push_str("**Implements:**\n");
                for trait_name in &node.traits {
                    report.push_str(&format!("- {}\n", trait_name));
                }
                report.push_str("\n");
            }
        }
    }

    report.push_str("\n## Recommendations\n\n");
    report.push_str("Based on the analysis, consider:\n\n");
    report.push_str("1. **Missing Aggregates**: Organization, Agent, Policy\n");
    report.push_str("2. **Missing Value Objects**: EmailAddress, PhoneNumber are referenced but not fully defined\n");
    report.push_str("3. **Missing Events**: Many commands don't have corresponding events\n");
    report.push_str("4. **Missing Handlers**: No command handlers or event handlers are defined\n");
    report.push_str("5. **Graph-specific types**: No graph-related aggregates (Graph, Node, Edge) are defined\n");

    report
}
