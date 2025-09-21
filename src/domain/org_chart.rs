// Copyright (c) 2025 - Cowboy AI, LLC.

//! OrgChart domain model: Project aggregate combining Organization, People,
//! Locations, and Policies into a cohesive aggregate. Pure, event-based API.

use crate::entity::EntityId;
use crate::errors::DomainError;
use crate::formal_domain::{DomainConcept, ValueObject};
use crate::DomainEvent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Marker types for identities (phantom typed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Marker type for Project IDs
pub struct ProjectMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Marker type for Organization IDs
pub struct OrganizationMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Marker type for Person IDs
pub struct PersonMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Marker type for Location IDs
pub struct LocationMarker;

/// Policy value object attached to projects
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PolicyVO {
    /// Policy name
    pub name: String,
}
impl DomainConcept for PolicyVO {}
impl ValueObject for PolicyVO {}

/// Lifecycle state for a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ProjectState {
    /// Initial state
    Proposed,
    /// Project is active
    Active,
    /// Project is completed
    Completed,
    /// Project is cancelled
    Cancelled,
}

/// Project Aggregate (pure in-memory representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAggregate {
    /// Project identity
    pub id: EntityId<ProjectMarker>,
    /// Lifecycle state
    pub state: ProjectState,
    /// Owning organization
    pub org: EntityId<OrganizationMarker>,
    /// Location of project
    pub location: EntityId<LocationMarker>,
    /// People and roles in the project
    pub members: Vec<(EntityId<PersonMarker>, String)>,
    /// Attached policies
    pub policies: Vec<PolicyVO>,
}

impl ProjectAggregate {
    /// Create a new Project in Proposed state
    pub fn new(
        id: EntityId<ProjectMarker>,
        org: EntityId<OrganizationMarker>,
        location: EntityId<LocationMarker>,
    ) -> Self {
        Self {
            id,
            state: ProjectState::Proposed,
            org,
            location,
            members: vec![],
            policies: vec![],
        }
    }
}

/// Commands to mutate ProjectAggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectCommand {
    /// Move Proposed -> Active
    Activate,
    /// Move Active -> Completed
    Complete,
    /// Move Proposed/Active -> Cancelled
    Cancel,
    /// Add a person with role
    /// Add a person with role
    AddMember {
        /// Person identity
        person: EntityId<PersonMarker>,
        /// Role within the project
        role: String,
    },
    /// Attach a policy value object
    /// Attach a policy value object
    AttachPolicy {
        /// Policy to attach
        policy: PolicyVO,
    },
}

/// Events produced by ProjectAggregate operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectEvent {
    /// Emitted when a project is initially created
    ProjectCreated {
        /// Project identity
        project_id: EntityId<ProjectMarker>,
        /// Owning organization
        org: EntityId<OrganizationMarker>,
        /// Location of project
        location: EntityId<LocationMarker>,
    },
    /// Emitted when project is activated
    ProjectActivated {
        /// Project identity
        project_id: EntityId<ProjectMarker>,
    },
    /// Emitted when project is completed
    ProjectCompleted {
        /// Project identity
        project_id: EntityId<ProjectMarker>,
    },
    /// Emitted when project is cancelled
    ProjectCancelled {
        /// Project identity
        project_id: EntityId<ProjectMarker>,
    },
    /// Emitted when a member is added
    MemberAdded {
        /// Project identity
        project_id: EntityId<ProjectMarker>,
        /// Person identity
        person: EntityId<PersonMarker>,
        /// Role name
        role: String,
    },
    /// Emitted when a policy is attached
    PolicyAttached {
        /// Project identity
        project_id: EntityId<ProjectMarker>,
        /// Attached policy
        policy: PolicyVO,
    },
}

impl DomainEvent for ProjectEvent {
    fn aggregate_id(&self) -> Uuid {
        match self {
            ProjectEvent::ProjectCreated { project_id, .. }
            | ProjectEvent::ProjectActivated { project_id }
            | ProjectEvent::ProjectCompleted { project_id }
            | ProjectEvent::ProjectCancelled { project_id }
            | ProjectEvent::MemberAdded { project_id, .. }
            | ProjectEvent::PolicyAttached { project_id, .. } => *project_id.as_uuid(),
        }
    }
    fn event_type(&self) -> &'static str {
        match self {
            ProjectEvent::ProjectCreated { .. } => "ProjectCreated",
            ProjectEvent::ProjectActivated { .. } => "ProjectActivated",
            ProjectEvent::ProjectCompleted { .. } => "ProjectCompleted",
            ProjectEvent::ProjectCancelled { .. } => "ProjectCancelled",
            ProjectEvent::MemberAdded { .. } => "MemberAdded",
            ProjectEvent::PolicyAttached { .. } => "PolicyAttached",
        }
    }
}

impl ProjectAggregate {
    /// Handle a command and return resulting events; updates self.
    pub fn handle(
        &mut self,
        cmd: ProjectCommand,
    ) -> Result<Vec<Box<dyn DomainEvent>>, DomainError> {
        use ProjectCommand::*;
        match (self.state, cmd) {
            (ProjectState::Proposed, Activate) => {
                self.state = ProjectState::Active;
                Ok(vec![Box::new(ProjectEvent::ProjectActivated {
                    project_id: self.id,
                })])
            }
            (ProjectState::Proposed, AddMember { person, role }) => {
                self.members.push((person, role.clone()));
                Ok(vec![Box::new(ProjectEvent::MemberAdded {
                    project_id: self.id,
                    person,
                    role,
                })])
            }
            (ProjectState::Proposed, AttachPolicy { policy }) => {
                self.policies.push(policy.clone());
                Ok(vec![Box::new(ProjectEvent::PolicyAttached {
                    project_id: self.id,
                    policy,
                })])
            }
            (ProjectState::Active, AddMember { person, role }) => {
                self.members.push((person, role.clone()));
                Ok(vec![Box::new(ProjectEvent::MemberAdded {
                    project_id: self.id,
                    person,
                    role,
                })])
            }
            (ProjectState::Active, AttachPolicy { policy }) => {
                self.policies.push(policy.clone());
                Ok(vec![Box::new(ProjectEvent::PolicyAttached {
                    project_id: self.id,
                    policy,
                })])
            }
            (ProjectState::Active, Complete) => {
                self.state = ProjectState::Completed;
                Ok(vec![Box::new(ProjectEvent::ProjectCompleted {
                    project_id: self.id,
                })])
            }
            (ProjectState::Proposed, Cancel) | (ProjectState::Active, Cancel) => {
                self.state = ProjectState::Cancelled;
                Ok(vec![Box::new(ProjectEvent::ProjectCancelled {
                    project_id: self.id,
                })])
            }
            // Invalid transitions
            (_, Activate)
            | (_, Complete)
            | (_, Cancel)
            | (_, AddMember { .. })
            | (_, AttachPolicy { .. }) => Err(DomainError::InvalidOperation {
                reason: "Invalid command in current state".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_flow_proposed_to_active_with_member_and_policy() {
        let pid = EntityId::<ProjectMarker>::new();
        let org = EntityId::<OrganizationMarker>::new();
        let loc = EntityId::<LocationMarker>::new();
        let mut p = ProjectAggregate::new(pid, org, loc);

        // Add member and policy in Proposed
        let person = EntityId::<PersonMarker>::new();
        let evs1 = p
            .handle(ProjectCommand::AddMember {
                person,
                role: "Lead".into(),
            })
            .unwrap();
        assert_eq!(evs1[0].event_type(), "MemberAdded");
        let evs2 = p
            .handle(ProjectCommand::AttachPolicy {
                policy: PolicyVO {
                    name: "Safety".into(),
                },
            })
            .unwrap();
        assert_eq!(evs2[0].event_type(), "PolicyAttached");

        // Activate
        let evs3 = p.handle(ProjectCommand::Activate).unwrap();
        assert_eq!(evs3[0].event_type(), "ProjectActivated");
        assert_eq!(p.state, ProjectState::Active);
    }
}
