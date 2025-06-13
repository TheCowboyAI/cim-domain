//! Policy aggregate - represents rules, constraints, and governance in the system
//!
//! Policies define what is allowed or required in the system. They can be simple
//! rules or complex workflows requiring approvals and external interactions.

use crate::{
    component::{Component, ComponentStorage},
    entity::{AggregateRoot, Entity, EntityId},
    errors::{DomainError, DomainResult},
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

/// Marker type for Policy entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PolicyMarker;

/// Policy aggregate root
#[derive(Debug, Clone)]
pub struct Policy {
    /// Entity base
    entity: Entity<PolicyMarker>,

    /// Policy type
    policy_type: PolicyType,

    /// Current status
    status: PolicyStatus,

    /// Policy scope (what it applies to)
    scope: PolicyScope,

    /// Owner (person, organization, or system)
    owner_id: Uuid,

    /// Components attached to this policy
    components: ComponentStorage,

    /// Version for optimistic concurrency
    version: u64,
}

/// Types of policies in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyType {
    /// Access control policy
    AccessControl,
    /// Data governance policy
    DataGovernance,
    /// Compliance policy (regulatory)
    Compliance,
    /// Operational policy
    Operational,
    /// Security policy
    Security,
    /// Approval workflow policy
    ApprovalWorkflow,
    /// Custom policy type
    Custom,
}

impl fmt::Display for PolicyType {
    /// Format the policy type for display
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyType::AccessControl => write!(f, "Access Control"),
            PolicyType::DataGovernance => write!(f, "Data Governance"),
            PolicyType::Compliance => write!(f, "Compliance"),
            PolicyType::Operational => write!(f, "Operational"),
            PolicyType::Security => write!(f, "Security"),
            PolicyType::ApprovalWorkflow => write!(f, "Approval Workflow"),
            PolicyType::Custom => write!(f, "Custom"),
        }
    }
}

/// Policy operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyStatus {
    /// Policy is being drafted
    Draft,
    /// Policy is pending approval
    PendingApproval,
    /// Policy is active and enforced
    Active,
    /// Policy is suspended temporarily
    Suspended,
    /// Policy has been superseded by another
    Superseded,
    /// Policy has been archived
    Archived,
}

/// What the policy applies to
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyScope {
    /// Applies globally to entire system
    Global,
    /// Applies to specific organization
    Organization(Uuid),
    /// Applies to specific context/domain
    Context(String),
    /// Applies to specific resource type
    ResourceType(String),
    /// Applies to specific entities
    Entities(HashSet<Uuid>),
    /// Custom scope with metadata
    Custom(HashMap<String, serde_json::Value>),
}

impl Policy {
    /// Create a new policy
    pub fn new(
        id: Uuid,
        policy_type: PolicyType,
        scope: PolicyScope,
        owner_id: Uuid,
    ) -> Self {
        Self {
            entity: Entity::with_id(EntityId::from_uuid(id)),
            policy_type,
            status: PolicyStatus::Draft,
            scope,
            owner_id,
            components: ComponentStorage::new(),
            version: 0,
        }
    }

    /// Get the policy's ID
    pub fn id(&self) -> Uuid {
        *self.entity.id.as_uuid()
    }

    /// Get the policy type
    pub fn policy_type(&self) -> PolicyType {
        self.policy_type
    }

    /// Get the current status
    pub fn status(&self) -> PolicyStatus {
        self.status
    }

    /// Get the policy scope
    pub fn scope(&self) -> &PolicyScope {
        &self.scope
    }

    /// Get the owner ID
    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    /// Add a component to the policy
    pub fn add_component<C: Component>(&mut self, component: C) -> DomainResult<()> {
        self.components.add(component)?;
        self.entity.touch();
        self.version += 1;
        Ok(())
    }

    /// Get a component by type
    pub fn get_component<C: Component>(&self) -> Option<&C> {
        self.components.get::<C>()
    }

    /// Remove a component by type
    pub fn remove_component<C: Component>(&mut self) -> Option<Box<dyn Component>> {
        let result = self.components.remove::<C>();
        if result.is_some() {
            self.entity.touch();
            self.version += 1;
        }
        result
    }

    /// Check if the policy has a specific component
    pub fn has_component<C: Component>(&self) -> bool {
        self.components.has::<C>()
    }

    /// Submit policy for approval
    pub fn submit_for_approval(&mut self) -> DomainResult<()> {
        match self.status {
            PolicyStatus::Draft => {
                self.status = PolicyStatus::PendingApproval;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "PendingApproval".to_string(),
            }),
        }
    }

    /// Approve the policy
    pub fn approve(&mut self) -> DomainResult<()> {
        match self.status {
            PolicyStatus::PendingApproval => {
                self.status = PolicyStatus::Active;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Active".to_string(),
            }),
        }
    }

    /// Reject the policy (back to draft)
    pub fn reject(&mut self, _reason: String) -> DomainResult<()> {
        match self.status {
            PolicyStatus::PendingApproval => {
                self.status = PolicyStatus::Draft;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Draft".to_string(),
            }),
        }
    }

    /// Suspend the policy
    pub fn suspend(&mut self, _reason: String) -> DomainResult<()> {
        match self.status {
            PolicyStatus::Active => {
                self.status = PolicyStatus::Suspended;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Suspended".to_string(),
            }),
        }
    }

    /// Reactivate a suspended policy
    pub fn reactivate(&mut self) -> DomainResult<()> {
        match self.status {
            PolicyStatus::Suspended => {
                self.status = PolicyStatus::Active;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Active".to_string(),
            }),
        }
    }

    /// Supersede this policy with another
    pub fn supersede(&mut self, _new_policy_id: Uuid) -> DomainResult<()> {
        match self.status {
            PolicyStatus::Active | PolicyStatus::Suspended => {
                self.status = PolicyStatus::Superseded;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Superseded".to_string(),
            }),
        }
    }

    /// Archive the policy
    pub fn archive(&mut self) -> DomainResult<()> {
        match self.status {
            PolicyStatus::Superseded | PolicyStatus::Suspended => {
                self.status = PolicyStatus::Archived;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Archived".to_string(),
            }),
        }
    }
}

impl AggregateRoot for Policy {
    type Id = EntityId<PolicyMarker>;

    fn id(&self) -> Self::Id {
        self.entity.id
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn increment_version(&mut self) {
        self.version += 1;
        self.entity.touch();
    }
}

// Policy Components

/// Rules component - defines the policy rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesComponent {
    /// Rule definitions (could be JSON, DSL, or structured data)
    pub rules: serde_json::Value,
    /// Rule engine type (e.g., "json-logic", "rego", "custom")
    pub engine: String,
    /// Rule version
    pub version: String,
}

impl Component for RulesComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "RulesComponent"
    }
}

/// Approval requirements component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequirementsComponent {
    /// Minimum number of approvals needed
    pub min_approvals: u32,
    /// Specific approvers required (by ID)
    pub required_approvers: HashSet<Uuid>,
    /// Approval roles (any person with these roles can approve)
    pub approval_roles: HashSet<String>,
    /// Approval timeout
    pub timeout: Option<chrono::Duration>,
    /// External approval requirements
    pub external_approvals: Vec<ExternalApprovalRequirement>,
}

/// External approval requirement for policies that need additional verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApprovalRequirement {
    /// Type of external approval (e.g., "yubikey", "biometric", "2fa")
    pub approval_type: String,
    /// Description of the requirement
    pub description: String,
    /// Metadata for the external system
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Component for ApprovalRequirementsComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ApprovalRequirementsComponent"
    }
}

/// Approval state component - tracks ongoing approvals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalStateComponent {
    /// Current approvals received
    pub approvals: Vec<Approval>,
    /// Pending external approvals
    pub pending_external: Vec<PendingExternalApproval>,
    /// Rejection reasons if any
    pub rejections: Vec<Rejection>,
    /// When approval process started
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Approval record for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    /// ID of the person who approved
    pub approver_id: Uuid,
    /// When the approval was given
    pub approved_at: chrono::DateTime<chrono::Utc>,
    /// Optional comments from the approver
    pub comments: Option<String>,
    /// External verification if required
    pub external_verification: Option<ExternalVerification>,
}

/// Pending external approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingExternalApproval {
    /// Type of approval required (e.g., "yubikey", "biometric")
    pub approval_type: String,
    /// Unique request identifier
    pub request_id: Uuid,
    /// When the approval was requested
    pub requested_at: chrono::DateTime<chrono::Utc>,
    /// When the approval request expires
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// External verification details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalVerification {
    /// Type of verification used (e.g., "yubikey", "biometric", "2fa")
    pub verification_type: String,
    /// Unique identifier from the verification system
    pub verification_id: String,
    /// When the verification was completed
    pub verified_at: chrono::DateTime<chrono::Utc>,
    /// Additional verification metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Rejection record for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rejection {
    /// ID of the person who rejected
    pub rejector_id: Uuid,
    /// When the rejection occurred
    pub rejected_at: chrono::DateTime<chrono::Utc>,
    /// Reason for rejection
    pub reason: String,
}

impl Component for ApprovalStateComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ApprovalStateComponent"
    }
}

/// Enforcement component - how the policy is enforced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementComponent {
    /// Enforcement mode
    pub mode: EnforcementMode,
    /// Actions to take on violation
    pub violation_actions: Vec<ViolationAction>,
    /// Exceptions to the policy
    pub exceptions: Vec<PolicyException>,
}

/// Mode of policy enforcement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementMode {
    /// Strictly enforce - block violations
    Strict,
    /// Log violations but allow
    Permissive,
    /// Dry run - simulate enforcement
    DryRun,
    /// Disabled - policy not enforced
    Disabled,
}

/// Action to take when a policy is violated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationAction {
    /// Type of action to take (e.g., "log", "alert", "block")
    pub action_type: String,
    /// Severity level of the violation
    pub severity: ViolationSeverity,
    /// IDs of entities to notify about the violation
    pub notification_targets: Vec<Uuid>,
}

/// Severity levels for policy violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Informational - no action required
    Info,
    /// Warning - should be investigated
    Warning,
    /// Error - requires action
    Error,
    /// Critical - immediate action required
    Critical,
}

/// Exception to a policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyException {
    /// Entity this exception applies to (if specific)
    pub entity_id: Option<Uuid>,
    /// Context where this exception applies
    pub context: Option<String>,
    /// Reason for the exception
    pub reason: String,
    /// When the exception expires
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Component for EnforcementComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "EnforcementComponent"
    }
}

/// Metadata component for policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    /// Human-readable name
    pub name: String,
    /// Description of the policy
    pub description: String,
    /// Policy category/tags
    pub tags: HashSet<String>,
    /// Effective date
    pub effective_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Expiration date
    pub expiration_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Compliance frameworks this policy supports
    pub compliance_frameworks: HashSet<String>,
}

impl Component for PolicyMetadata {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "PolicyMetadata"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_policy() {
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let policy = Policy::new(
            policy_id,
            PolicyType::AccessControl,
            PolicyScope::Global,
            owner_id,
        );

        assert_eq!(policy.id(), policy_id);
        assert_eq!(policy.policy_type(), PolicyType::AccessControl);
        assert_eq!(policy.status(), PolicyStatus::Draft);
        assert_eq!(policy.owner_id(), owner_id);
        assert_eq!(policy.version(), 0);
    }

    #[test]
    fn test_policy_approval_workflow() {
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let mut policy = Policy::new(
            policy_id,
            PolicyType::Compliance,
            PolicyScope::Global,
            owner_id,
        );

        // Cannot approve draft directly
        assert!(policy.approve().is_err());

        // Submit for approval
        assert!(policy.submit_for_approval().is_ok());
        assert_eq!(policy.status(), PolicyStatus::PendingApproval);
        assert_eq!(policy.version(), 1);

        // Approve
        assert!(policy.approve().is_ok());
        assert_eq!(policy.status(), PolicyStatus::Active);
        assert_eq!(policy.version(), 2);

        // Cannot submit active policy for approval
        assert!(policy.submit_for_approval().is_err());
    }

    #[test]
    fn test_policy_rejection_flow() {
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let mut policy = Policy::new(
            policy_id,
            PolicyType::Security,
            PolicyScope::Global,
            owner_id,
        );

        // Submit for approval
        assert!(policy.submit_for_approval().is_ok());
        assert_eq!(policy.status(), PolicyStatus::PendingApproval);

        // Reject
        assert!(policy.reject("Needs more detail".to_string()).is_ok());
        assert_eq!(policy.status(), PolicyStatus::Draft);
        assert_eq!(policy.version(), 2);
    }

    #[test]
    fn test_policy_suspension_and_reactivation() {
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let mut policy = Policy::new(
            policy_id,
            PolicyType::Operational,
            PolicyScope::Global,
            owner_id,
        );

        // Get to active state
        policy.submit_for_approval().unwrap();
        policy.approve().unwrap();

        // Suspend
        assert!(policy.suspend("Temporary issue".to_string()).is_ok());
        assert_eq!(policy.status(), PolicyStatus::Suspended);

        // Reactivate
        assert!(policy.reactivate().is_ok());
        assert_eq!(policy.status(), PolicyStatus::Active);

        // Cannot reactivate active policy
        assert!(policy.reactivate().is_err());
    }

    #[test]
    fn test_policy_supersede_and_archive() {
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let new_policy_id = Uuid::new_v4();

        let mut policy = Policy::new(
            policy_id,
            PolicyType::DataGovernance,
            PolicyScope::Global,
            owner_id,
        );

        // Get to active state
        policy.submit_for_approval().unwrap();
        policy.approve().unwrap();

        // Supersede
        assert!(policy.supersede(new_policy_id).is_ok());
        assert_eq!(policy.status(), PolicyStatus::Superseded);

        // Archive
        assert!(policy.archive().is_ok());
        assert_eq!(policy.status(), PolicyStatus::Archived);

        // Cannot reactivate archived policy
        assert!(policy.reactivate().is_err());
    }

    #[test]
    fn test_policy_components() {
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let mut policy = Policy::new(
            policy_id,
            PolicyType::ApprovalWorkflow,
            PolicyScope::Global,
            owner_id,
        );

        // Add metadata
        let metadata = PolicyMetadata {
            name: "Data Access Policy".to_string(),
            description: "Controls access to sensitive data".to_string(),
            tags: ["security", "data", "compliance"].iter().map(|s| s.to_string()).collect(),
            effective_date: Some(chrono::Utc::now()),
            expiration_date: None,
            compliance_frameworks: ["SOC2", "GDPR"].iter().map(|s| s.to_string()).collect(),
        };
        assert!(policy.add_component(metadata).is_ok());

        // Add approval requirements
        let approval_reqs = ApprovalRequirementsComponent {
            min_approvals: 2,
            required_approvers: HashSet::new(),
            approval_roles: ["security_admin", "compliance_officer"].iter().map(|s| s.to_string()).collect(),
            timeout: Some(chrono::Duration::days(7)),
            external_approvals: vec![
                ExternalApprovalRequirement {
                    approval_type: "yubikey".to_string(),
                    description: "Yubikey touch required for approval".to_string(),
                    metadata: HashMap::new(),
                },
            ],
        };
        assert!(policy.add_component(approval_reqs).is_ok());

        // Verify components
        assert!(policy.has_component::<PolicyMetadata>());
        assert!(policy.has_component::<ApprovalRequirementsComponent>());

        let reqs = policy.get_component::<ApprovalRequirementsComponent>().unwrap();
        assert_eq!(reqs.min_approvals, 2);
        assert_eq!(reqs.external_approvals.len(), 1);
    }

    #[test]
    fn test_approval_state_tracking() {
        let mut approval_state = ApprovalStateComponent {
            approvals: vec![],
            pending_external: vec![],
            rejections: vec![],
            started_at: chrono::Utc::now(),
        };

        // Add pending external approval
        approval_state.pending_external.push(PendingExternalApproval {
            approval_type: "yubikey".to_string(),
            request_id: Uuid::new_v4(),
            requested_at: chrono::Utc::now(),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::minutes(5)),
        });

        // Add approval with external verification
        approval_state.approvals.push(Approval {
            approver_id: Uuid::new_v4(),
            approved_at: chrono::Utc::now(),
            comments: Some("Approved with yubikey verification".to_string()),
            external_verification: Some(ExternalVerification {
                verification_type: "yubikey".to_string(),
                verification_id: "YK123456".to_string(),
                verified_at: chrono::Utc::now(),
                metadata: HashMap::new(),
            }),
        });

        assert_eq!(approval_state.approvals.len(), 1);
        assert_eq!(approval_state.pending_external.len(), 1);
        assert!(approval_state.approvals[0].external_verification.is_some());
    }

    #[test]
    fn test_policy_scopes() {
        let policy_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        // Organization-scoped policy
        let org_policy = Policy::new(
            policy_id,
            PolicyType::AccessControl,
            PolicyScope::Organization(org_id),
            owner_id,
        );

        match org_policy.scope() {
            PolicyScope::Organization(id) => assert_eq!(*id, org_id),
            _ => panic!("Expected organization scope"),
        }

        // Entity-specific policy
        let entity_ids: HashSet<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let entity_policy = Policy::new(
            Uuid::new_v4(),
            PolicyType::Security,
            PolicyScope::Entities(entity_ids.clone()),
            owner_id,
        );

        match entity_policy.scope() {
            PolicyScope::Entities(ids) => assert_eq!(ids.len(), 3),
            _ => panic!("Expected entities scope"),
        }
    }
}
