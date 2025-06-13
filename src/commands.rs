//! Domain commands for CIM
//!
//! Commands represent requests to change state. They are processed by command handlers
//! which validate business rules and emit events. Commands return only acknowledgments,
//! not data - use queries for data retrieval.

// All domain-specific commands have been moved to their respective domain submodules:
// - Person commands: cim-domain-person
// - Organization commands: cim-domain-organization
// - Agent commands: cim-domain-agent
// - Workflow commands: cim-domain-workflow
// - Location commands: cim-domain-location
// - Document commands: cim-domain-document
// - Policy commands: cim-domain-policy

