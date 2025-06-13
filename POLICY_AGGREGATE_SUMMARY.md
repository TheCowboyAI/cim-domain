# Policy Aggregate Implementation Summary

## Overview

The Policy aggregate has been fully implemented in the `cim-domain` crate, following Domain-Driven Design (DDD) patterns. Policies represent rules, constraints, and governance in the CIM system, with special support for approval workflows and external interactions (like yubikey touches or biometric confirmations).

## Key Features

### 1. Policy Types
```rust
pub enum PolicyType {
    AccessControl,      // Access control policy
    DataGovernance,     // Data governance policy
    Compliance,         // Regulatory compliance
    Operational,        // Operational policy
    Security,           // Security policy
    ApprovalWorkflow,   // Approval workflow policy
    Custom,             // Custom policy type
}
```

### 2. Policy Status State Machine
```rust
pub enum PolicyStatus {
    Draft,              // Policy is being drafted
    PendingApproval,    // Policy is pending approval
    Active,             // Policy is active and enforced
    Suspended,          // Policy is temporarily suspended
    Superseded,         // Policy has been superseded by another
    Archived,           // Policy has been archived
}
```

Valid transitions:
- Draft → PendingApproval (submit for approval)
- PendingApproval → Active (approve)
- PendingApproval → Draft (reject)
- Active → Suspended (suspend)
- Suspended → Active (reactivate)
- Active/Suspended → Superseded (supersede)
- Superseded/Suspended → Archived (archive)

### 3. Policy Scope
```rust
pub enum PolicyScope {
    Global,                              // Applies globally
    Organization(Uuid),                  // Applies to specific organization
    Context(String),                     // Applies to specific context/domain
    ResourceType(String),                // Applies to specific resource type
    Entities(HashSet<Uuid>),            // Applies to specific entities
    Custom(HashMap<String, Value>),      // Custom scope with metadata
}
```

### 4. Component System

The Policy aggregate uses a component-based architecture for extensibility:

#### RulesComponent
- Defines the policy rules (JSON, DSL, or structured data)
- Specifies rule engine type (e.g., "json-logic", "rego", "custom")
- Includes rule version

#### ApprovalRequirementsComponent
- Minimum number of approvals needed
- Specific approvers required (by ID)
- Approval roles (any person with these roles can approve)
- Approval timeout
- **External approval requirements** (yubikey, biometric, 2FA, etc.)

#### ApprovalStateComponent
- Tracks current approvals received
- Manages pending external approvals
- Records rejections with reasons
- Tracks when approval process started

#### EnforcementComponent
- Enforcement mode (Strict, Permissive, DryRun, Disabled)
- Actions to take on violation
- Policy exceptions

#### PolicyMetadata
- Human-readable name and description
- Tags for categorization
- Effective and expiration dates
- Compliance frameworks supported

### 5. External Approval Support

The Policy aggregate has first-class support for external approvals:

```rust
pub struct ExternalApprovalRequirement {
    pub approval_type: String,      // e.g., "yubikey", "biometric", "2fa"
    pub description: String,
    pub metadata: HashMap<String, Value>,
}

pub struct ExternalVerification {
    pub verification_type: String,
    pub verification_id: String,
    pub verified_at: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}
```

This enables workflows where:
1. Policy requires yubikey touch for approval
2. System sends request for external approval
3. User touches yubikey
4. External system sends verification event
5. Policy records the external verification
6. Policy becomes active once all approvals are received

## Domain Events

The Policy aggregate emits the following events:

- **PolicyEnacted**: New policy created
- **PolicySubmittedForApproval**: Policy submitted for approval
- **PolicyApproved**: Policy approved (with optional external verification)
- **PolicyRejected**: Policy rejected with reason
- **PolicySuspended**: Policy temporarily suspended
- **PolicyReactivated**: Suspended policy reactivated
- **PolicySuperseded**: Policy replaced by another
- **PolicyArchived**: Policy archived
- **PolicyExternalApprovalRequested**: External approval requested
- **PolicyExternalApprovalReceived**: External approval received with verification

## Commands

The Policy aggregate handles these commands:

- **EnactPolicy**: Create a new policy
- **SubmitPolicyForApproval**: Submit draft for approval
- **ApprovePolicy**: Approve a policy (with optional external verification)
- **RejectPolicy**: Reject a policy with reason
- **SuspendPolicy**: Temporarily suspend a policy
- **ReactivatePolicy**: Reactivate a suspended policy
- **SupersedePolicy**: Replace with another policy
- **ArchivePolicy**: Archive a policy
- **RequestPolicyExternalApproval**: Request external approval
- **RecordPolicyExternalApproval**: Record external approval received
- **UpdatePolicyRules**: Update policy rules

## Usage Example

```rust
// Create a new policy requiring yubikey approval
let policy_id = Uuid::new_v4();
let mut policy = Policy::new(
    policy_id,
    PolicyType::Security,
    PolicyScope::Global,
    owner_id,
);

// Add metadata
policy.add_component(PolicyMetadata {
    name: "Sensitive Data Access Policy".to_string(),
    description: "Requires yubikey verification for access".to_string(),
    tags: ["security", "data-access", "yubikey"].into(),
    effective_date: Some(Utc::now()),
    expiration_date: None,
    compliance_frameworks: ["SOC2", "ISO27001"].into(),
});

// Add approval requirements with yubikey
policy.add_component(ApprovalRequirementsComponent {
    min_approvals: 2,
    required_approvers: HashSet::new(),
    approval_roles: ["security_admin", "compliance_officer"].into(),
    timeout: Some(Duration::days(7)),
    external_approvals: vec![
        ExternalApprovalRequirement {
            approval_type: "yubikey".to_string(),
            description: "Yubikey touch required for approval".to_string(),
            metadata: HashMap::new(),
        },
    ],
});

// Submit for approval
policy.submit_for_approval()?;

// When yubikey is touched, record the external approval
let verification = ExternalVerification {
    verification_type: "yubikey".to_string(),
    verification_id: "YK123456".to_string(),
    verified_at: Utc::now(),
    metadata: [("serial", "123456")].into(),
};

// Approve with external verification
policy.approve()?;
```

## Integration with CIM

The Policy aggregate integrates seamlessly with the CIM architecture:

1. **Event-Driven**: All state changes emit domain events
2. **Component-Based**: Extensible through components
3. **NATS Integration**: Events use proper subject routing (`policies.policy.event_type.v1`)
4. **Bevy Bridge**: Policies can be visualized in the Bevy ECS
5. **External Interactions**: First-class support for external approval mechanisms

## Test Coverage

The Policy aggregate includes 8 comprehensive tests covering:
- Policy creation
- Approval workflow
- Rejection flow
- Suspension and reactivation
- Superseding and archiving
- Component management
- Approval state tracking
- Policy scopes

## Next Steps

With all core aggregates now implemented, the next priorities are:
1. Implement command handlers to process Policy commands
2. Create event handlers for external approval workflows
3. Build integration tests showing Policy + Agent interactions
4. Create examples of policies requiring external approvals
