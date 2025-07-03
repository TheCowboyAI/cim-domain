//! State Machine Demo
//!
//! This demo shows state machine transitions for all aggregates,
//! demonstrating how domain entities move through their lifecycles.

use cim_domain::{
    // Aggregates and types
    Agent, AgentType, AgentStatus,
    Policy, PolicyType, PolicyScope, PolicyStatus, PolicyMetadata,
    Document, DocumentStatus, DocumentInfoComponent, EntityId, DocumentMarker,
    // For CID creation
    DomainError,
};
use chrono::Utc;
use std::collections::{HashSet, HashMap};
use uuid::Uuid;
use cid::Cid;

/// Demo showing state transitions
struct StateMachineDemo {
    agents: Vec<Agent>,
    policies: Vec<Policy>,
    documents: Vec<Document>,
}

impl StateMachineDemo {
    fn new() -> Self {
        Self {
            agents: Vec::new(),
            policies: Vec::new(),
            documents: Vec::new(),
        }
    }

    /// Demonstrate agent state transitions
    fn demo_agent_states(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Agent State Machine Demo ===\n");

        // Create an agent
        let agent_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let mut agent = Agent::new(agent_id, AgentType::AI, owner_id);

        println!("1. Agent created in Initializing state");
        println!("   Status: {:?}", agent.status());

        // Activate the agent
        agent.activate()?;
        println!("\n2. Agent activated");
        println!("   Status: {:?}", agent.status());

        // Suspend the agent
        agent.suspend("Maintenance required".to_string())?;
        println!("\n3. Agent suspended");
        println!("   Status: {:?}", agent.status());

        // Reactivate
        agent.activate()?;
        println!("\n4. Agent reactivated");
        println!("   Status: {:?}", agent.status());

        // Set offline
        agent.set_offline()?;
        println!("\n5. Agent went offline");
        println!("   Status: {:?}", agent.status());

        // Try to activate from offline
        agent.activate()?;
        println!("\n6. Agent back online");
        println!("   Status: {:?}", agent.status());

        // Decommission
        agent.decommission()?;
        println!("\n7. Agent decommissioned");
        println!("   Status: {:?}", agent.status());

        // Try invalid transition
        println!("\n8. Attempting invalid transition (decommissioned -> active)");
        match agent.activate() {
            Err(e) => println!("   Error (expected): {e}"),
            Ok(_) => println!("   Unexpected success!"),
        }

        self.agents.push(agent);
        Ok(())
    }

    /// Demonstrate policy state transitions
    fn demo_policy_states(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n\n=== Policy State Machine Demo ===\n");

        // Create a policy
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let mut policy = Policy::new(
            policy_id,
            PolicyType::Security,
            PolicyScope::Organization(owner_id),
            owner_id,
        );

        // Add metadata as a component
        let metadata = PolicyMetadata {
            name: "Security Policy".to_string(),
            description: "Organization security policy".to_string(),
            tags: ["security", "compliance"].iter().map(|s| s.to_string()).collect(),
            effective_date: Some(Utc::now()),
            expiration_date: None,
            compliance_frameworks: ["SOC2", "ISO27001"].iter().map(|s| s.to_string()).collect(),
        };
        policy.add_component(metadata)?;

        println!("1. Policy created in Draft state");
        println!("   Status: {:?}", policy.status());

        // Submit for approval
        policy.submit_for_approval()?;
        println!("\n2. Policy submitted for approval");
        println!("   Status: {:?}", policy.status());

        // Approve the policy
        policy.approve()?;
        println!("\n3. Policy approved");
        println!("   Status: {:?}", policy.status());

        // Suspend the policy
        policy.suspend("Review required".to_string())?;
        println!("\n4. Policy suspended");
        println!("   Status: {:?}", policy.status());

        // Reactivate
        policy.reactivate()?;
        println!("\n5. Policy reactivated");
        println!("   Status: {:?}", policy.status());

        // Create another policy to supersede
        let new_policy_id = Uuid::new_v4();
        policy.supersede(new_policy_id)?;
        println!("\n6. Policy superseded by new version");
        println!("   Status: {:?}", policy.status());

        // Try to reactivate superseded policy
        println!("\n7. Attempting to reactivate superseded policy");
        match policy.reactivate() {
            Err(e) => println!("   Error (expected): {e}"),
            Ok(_) => println!("   Unexpected success!"),
        }

        self.policies.push(policy);

        // Demo rejection flow
        let mut policy2 = Policy::new(
            Uuid::new_v4(),
            PolicyType::DataGovernance,
            PolicyScope::Global,
            owner_id,
        );

        let metadata2 = PolicyMetadata {
            name: "Data Policy".to_string(),
            description: "Data governance policy".to_string(),
            tags: HashSet::new(),
            effective_date: None,
            expiration_date: None,
            compliance_frameworks: HashSet::new(),
        };
        policy2.add_component(metadata2)?;

        policy2.submit_for_approval()?;
        policy2.reject("Needs more detail".to_string())?;
        println!("\n8. Second policy rejected");
        println!("   Status: {:?}", policy2.status());

        self.policies.push(policy2);
        Ok(())
    }

    /// Demonstrate document state transitions
    fn demo_document_states(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n\n=== Document State Machine Demo ===\n");

        // Create a document
        let doc_id = EntityId::<DocumentMarker>::new();
        let info = DocumentInfoComponent {
            title: "Technical Specification".to_string(),
            description: Some("System architecture document".to_string()),
            mime_type: "application/pdf".to_string(),
            filename: Some("tech_spec.pdf".to_string()),
            size_bytes: 1024 * 1024, // 1MB
            language: Some("en".to_string()),
        };

        // Create a dummy CID for the content
        let content_cid = Cid::default();
        let mut document = Document::new(doc_id, info, content_cid);

        println!("1. Document created");
        println!("   Has info component: {document.has_component::<DocumentInfoComponent>(}"));

        // Add lifecycle component to track status
        use cim_domain::{LifecycleComponent, DocumentStatus};
        let lifecycle = LifecycleComponent {
            status: DocumentStatus::Draft,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            version_number: "1.0".to_string(),
            previous_version_cid: None,
            expires_at: None,
            retention_policy: Some("7 years".to_string()),
        };
        document.add_component(lifecycle, "system", Some("Initial lifecycle".to_string()))?;

        if let Some(lc) = document.get_component::<LifecycleComponent>() {
            println!("   Status: {:?}", lc.status);
        }

        // Update status to under review
        document.remove_component::<LifecycleComponent>()?;
        let mut lifecycle = LifecycleComponent {
            status: DocumentStatus::UnderReview,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            version_number: "1.0".to_string(),
            previous_version_cid: None,
            expires_at: None,
            retention_policy: Some("7 years".to_string()),
        };
        document.add_component(lifecycle, "reviewer", Some("Document under review".to_string()))?;

        println!("\n2. Document under review");
        if let Some(lc) = document.get_component::<LifecycleComponent>() {
            println!("   Status: {:?}", lc.status);
        }

        // Publish the document
        document.remove_component::<LifecycleComponent>()?;
        lifecycle = LifecycleComponent {
            status: DocumentStatus::Published,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            version_number: "1.0".to_string(),
            previous_version_cid: None,
            expires_at: None,
            retention_policy: Some("7 years".to_string()),
        };
        document.add_component(lifecycle, "publisher", Some("Document published".to_string()))?;

        println!("\n3. Document published");
        if let Some(lc) = document.get_component::<LifecycleComponent>() {
            println!("   Status: {:?}", lc.status);
        }

        // Archive the document
        document.remove_component::<LifecycleComponent>()?;
        lifecycle = LifecycleComponent {
            status: DocumentStatus::Archived,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            version_number: "1.0".to_string(),
            previous_version_cid: None,
            expires_at: None,
            retention_policy: Some("7 years".to_string()),
        };
        document.add_component(lifecycle, "archiver", Some("End of active use".to_string()))?;

        println!("\n4. Document archived");
        if let Some(lc) = document.get_component::<LifecycleComponent>() {
            println!("   Status: {:?}", lc.status);
        }

        self.documents.push(document);

        // Demo superseded flow
        let doc2_id = EntityId::<DocumentMarker>::new();
        let info2 = DocumentInfoComponent {
            title: "Updated Specification".to_string(),
            description: Some("Version 2 of the system architecture".to_string()),
            mime_type: "application/pdf".to_string(),
            filename: Some("tech_spec_v2.pdf".to_string()),
            size_bytes: 2 * 1024 * 1024, // 2MB
            language: Some("en".to_string()),
        };

        let mut doc2 = Document::new(doc2_id, info2, Cid::default());

        let lifecycle2 = LifecycleComponent {
            status: DocumentStatus::Published,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            version_number: "2.0".to_string(),
            previous_version_cid: Some(content_cid), // Reference to v1
            expires_at: None,
            retention_policy: Some("7 years".to_string()),
        };
        doc2.add_component(lifecycle2, "publisher", Some("New version published".to_string()))?;

        println!("\n5. New document version created");
        if let Some(lc) = doc2.get_component::<LifecycleComponent>() {
            println!("   Status: {:?}", lc.status);
            println!("   Version: {lc.version_number}");
            println!("   Supersedes previous: {lc.previous_version_cid.is_some(}"));
        }

        self.documents.push(doc2);
        Ok(())
    }

    /// Show summary of all state machines
    fn show_summary(&self) {
        println!("\n\n=== State Machine Summary ===\n");

        println!("Agent States:");
        println!("  Initializing → Active → Suspended → Active");
        println!("  Active → Offline → Active");
        println!("  Any → Decommissioned (terminal)");

        println!("\nPolicy States:");
        println!("  Draft → PendingApproval → Active");
        println!("  PendingApproval → Draft (rejection)");
        println!("  Active → Suspended → Active");
        println!("  Active → Superseded (terminal)");
        println!("  Superseded → Archived (terminal)");

        println!("\nDocument States (via LifecycleComponent):");
        println!("  Draft → UnderReview → Published");
        println!("  Published → Archived");
        println!("  Published → Superseded (by new version)");

        println!("\nDemonstrated Entities:");
        println!("  - {self.agents.len(} Agents"));
        println!("  - {self.policies.len(} Policies"));
        println!("  - {self.documents.len(} Documents"));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CIM State Machine Demo ===\n");
    println!("This demo shows state transitions for domain aggregates.\n");

    let mut demo = StateMachineDemo::new();

    // Run state machine demos
    demo.demo_agent_states()?;
    demo.demo_policy_states()?;
    demo.demo_document_states()?;

    // Show summary
    demo.show_summary();

    println!("\n=== Demo Complete ===");
    println!("\nKey Takeaways:");
    println!("• Each aggregate has well-defined state transitions");
    println!("• Invalid transitions are prevented by the domain model");
    println!("• Terminal states cannot be exited");
    println!("• State changes generate domain events (in full implementation)");
    println!("• Business rules are enforced at the aggregate level");

    Ok(())
}
