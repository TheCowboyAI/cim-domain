//! Workflow module implementing category theory-based workflows with injectable states
//!
//! This module provides a flexible workflow system where:
//! - States are fully injectable by users (not hardcoded)
//! - Workflows form categories with states as objects and transitions as morphisms
//! - Enrichment captures business value, costs, and semantic meaning
//! - Integration with NATS for event-driven execution

pub mod category;
pub mod state;
pub mod transition;
pub mod aggregate;
pub mod commands;
pub mod events;

pub use category::*;
pub use state::*;
pub use transition::*;
pub use aggregate::*;
pub use commands::*;
pub use events::*;

// Re-export workflow events from domain_events
pub use crate::domain_events::{
    WorkflowStarted, WorkflowTransitionExecuted, WorkflowCompleted,
    WorkflowSuspended, WorkflowResumed, WorkflowCancelled, WorkflowFailed,
};
