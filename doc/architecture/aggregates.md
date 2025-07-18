# CIM Domain Aggregates

This document provides comprehensive documentation of all domain aggregates in the CIM Domain model. Each aggregate represents a core business concept with its own consistency boundary, commands, events, and business rules.

## Overview

The CIM Domain implements seven core aggregates that form the foundation of the system:

1. **Person** - Human actors with identity and roles
2. **Organization** - Groups, companies, and collective entities
3. **Agent** - AI and automated entities with bounded capabilities
4. **Location** - Physical, virtual, and logical spaces
5. **Policy** - Governance rules and approval workflows
6. **Document** - Files, content, and media (planned)
7. **Workflow** - Business processes and orchestration (planned)

## Person Aggregate

### Overview

The Person aggregate represents individual human actors in the system. It uses a component-based architecture for extensibility, allowing dynamic addition of identity, contact, employment, and other components.

### Structure

```rust
pub struct Person {
    entity: Entity<PersonMarker>,
    version: u64,
    components: ComponentStorage,
}
```

### Commands

| Command | Description | Key Fields |
|---------|-------------|------------|
| `RegisterPerson` | Creates a new person | location_id, identity components |
| `UpdatePersonProfile` | Updates person data | location_id, component changes |
| `AddPersonComponent` | Adds a new component | component type and data |
| `RemovePersonComponent` | Removes a component | component ID |

### Events

- `PersonRegistered` - Initial person creation with location reference
- `PersonComponentAdded` - Component attached to person
- `PersonComponentRemoved` - Component detached from person

### Components

#### IdentityComponent
```rust
- legal_name: String
- preferred_name: Option<String>
- date_of_birth: Option<NaiveDate>
- government_id: Option<String>
```

#### ContactComponent
```rust
- emails: Vec<EmailAddress>
- phones: Vec<PhoneNumber>
- addresses: Vec<Uuid>  // Location references
```

#### EmploymentComponent
```rust
- organization_id: Uuid
- employee_id: String
- title: String
- department: Option<String>
- manager_id: Option<Uuid>
- status: EmploymentStatus
- start_date: DateTime<Utc>
- end_date: Option<DateTime<Utc>>
```

#### ExternalIdentifiersComponent
Maps external system identities:
- LDAP DN
- Active Directory SID
- OAuth subjects
- Custom system IDs

### Business Rules

- Components are immutable (replaced, not mutated)
- Version tracking prevents concurrent update conflicts
- Component metadata tracks who added it and when
- Required components validated for specific views (e.g., LDAP projection)

## Organization Aggregate

### Overview

The Organization aggregate models collective entities from small teams to large corporations, supporting hierarchical structures, member management, and location associations.

### Structure

```rust
pub struct Organization {
    entity: Entity<OrganizationMarker>,
    version: u64,
    name: String,
    org_type: OrganizationType,
    status: OrganizationStatus,
    parent_id: Option<EntityId<OrganizationMarker>>,
    child_units: HashSet<EntityId<OrganizationMarker>>,
    members: HashMap<Uuid, HashSet<OrganizationRole>>,
    locations: HashSet<Uuid>,
    primary_location: Option<Uuid>,
    components: ComponentStorage,
}
```

### Commands

| Command | Description |
|---------|-------------|
| `CreateOrganization` | Establishes new organization |
| `AddOrganizationMember` | Adds person with roles |
| `UpdateOrganizationStructure` | Modifies hierarchy |
| `SetOrganizationStatus` | Changes operational status |
| `AssignOrganizationLocation` | Associates locations |

### Events

- Lifecycle: `OrganizationCreated`, `OrganizationStatusChanged`
- Members: `OrganizationMemberAdded/Removed`, `MemberRoleAssigned/Removed`
- Structure: `OrganizationParentSet/Removed`, `OrganizationChildUnitsAdded/Removed`
- Locations: `OrganizationLocationsAdded/Removed`, `PrimaryLocationSet/Removed`

### State Machine

```
Active ←→ Suspended
  ↓         ↓
Inactive → Dissolved
```

### Value Objects

#### OrganizationType
- Company, Department, Team, Division, Subsidiary
- NonProfit, Government, Partnership, Custom

#### OrganizationRole
```rust
- title: String
- level: OrganizationLevel  // Executive → Intern
- department: Option<String>
- is_manager: bool
```

### Components

- **OrganizationMetadata**: Industry, size, website, founding date
- **BudgetComponent**: Fiscal year, budget, currency, allocations

### Business Rules

- Prevents circular references in organizational hierarchy
- Enforces role-level hierarchy (executives > managers > staff)
- Primary location must be in organization's location set
- Parent-child relationships maintain bidirectional consistency

## Agent Aggregate

### Overview

The Agent aggregate represents automated actors (AI, bots, services) with defined capabilities, permissions, and operational states.

### Structure

```rust
pub struct Agent {
    entity: Entity<AgentMarker>,
    version: u64,
    agent_type: AgentType,
    owner_id: Uuid,
    status: AgentStatus,
    components: ComponentStorage,
}
```

### Commands

| Command | Description |
|---------|-------------|
| `DeployAgent` | Creates and initializes agent |
| `ActivateAgent` | Brings agent online |
| `SuspendAgent` | Temporarily disables |
| `SetAgentOffline` | Marks as unavailable |
| `DecommissionAgent` | Permanently retires |
| `UpdateAgentCapabilities` | Modifies abilities |
| `GrantAgentPermissions` | Adds access rights |
| `EnableAgentTools` | Provides tool access |

### State Machine

```
Initializing → Active
     ↓           ↓ ↑
     ↓      Suspended
     ↓           ↓
     ↓        Offline
     ↓           ↓
     └→ Decommissioned ←┘
```

### Value Objects

#### AgentType
- Human (human-operated agent)
- AI (artificial intelligence)
- System (automated service)
- External (third-party integration)

### Components

#### CapabilitiesComponent
```rust
- capabilities: HashMap<String, CapabilityMetadata>
- version: String
```

#### AuthenticationComponent
```rust
- methods: Vec<AuthMethod>  // ApiKey, OAuth2, JWT, Certificate
- credentials: HashMap<String, SecureString>
```

#### PermissionsComponent
```rust
- granted: HashSet<String>
- denied: HashSet<String>
- roles: HashSet<String>
```

#### ToolAccessComponent
```rust
- tools: HashMap<String, ToolDefinition>
- usage_stats: HashMap<String, ToolUsageStats>
```

### Business Rules

- State transitions follow defined paths (no skipping states)
- Decommissioned state is terminal
- Capabilities versioned for compatibility
- Permissions follow principle of least privilege

## Location Aggregate

### Overview

The Location aggregate models physical addresses, virtual spaces, and logical locations with support for hierarchical relationships and rich metadata.

### Structure

```rust
pub struct Location {
    entity: Entity<LocationMarker>,
    version: u64,
    name: String,
    location_type: LocationType,
    address: Option<Address>,
    coordinates: Option<GeoCoordinates>,
    virtual_location: Option<VirtualLocation>,
    parent_id: Option<EntityId<LocationMarker>>,
    metadata: HashMap<String, String>,
}
```

### Commands

| Command | Description |
|---------|-------------|
| `DefineLocation` | Creates any type of location |

### Events

- `LocationDefined` - Complete location specification

### Location Types

```rust
enum LocationType {
    Physical,   // Real-world locations
    Virtual,    // Online/digital spaces
    Logical,    // Conceptual locations
    Hybrid,     // Mixed physical/virtual
}
```

### Value Objects

#### Address
```rust
pub struct Address {
    street1: String,
    street2: Option<String>,
    locality: String,      // City
    region: String,        // State/Province
    country: String,       // ISO code
    postal_code: String,
}
```

#### GeoCoordinates
```rust
pub struct GeoCoordinates {
    latitude: f64,         // -90 to 90
    longitude: f64,        // -180 to 180
    altitude: Option<f64>, // Meters
    coordinate_system: String, // Default: WGS84
}
```

#### VirtualLocation
```rust
pub struct VirtualLocation {
    platform: String,      // zoom, teams, metaverse
    platform_id: String,   // Room/space ID
    url: Option<String>,
    platform_data: HashMap<String, Value>,
}
```

### Business Rules

- Virtual locations cannot have physical addresses or coordinates
- Coordinates validated within valid ranges
- Parent location references prevent self-reference
- Address fields must be non-empty (except street2)
- Distance calculations use Haversine formula

## Policy Aggregate

### Overview

The Policy aggregate implements governance rules, approval workflows, and compliance requirements with support for external verification systems.

### Structure

```rust
pub struct Policy {
    entity: Entity<PolicyMarker>,
    version: u64,
    policy_type: PolicyType,
    scope: PolicyScope,
    status: PolicyStatus,
    owner_id: Uuid,
    components: ComponentStorage,
}
```

### Commands

| Command | Description |
|---------|-------------|
| `EnactPolicy` | Creates new policy |
| `SubmitPolicyForApproval` | Initiates approval |
| `ApprovePolicy` | Records approval |
| `RejectPolicy` | Denies policy |
| `SuspendPolicy` | Temporarily disables |
| `ReactivatePolicy` | Re-enables policy |
| `SupersedePolicy` | Replaces with new version |
| `ArchivePolicy` | Permanently retires |
| `RequestPolicyExternalApproval` | Initiates external verification |
| `RecordPolicyExternalApproval` | Records external approval |

### State Machine

```
Draft → PendingApproval → Active → Suspended
  ↑           ↓             ↓         ↓
  └─ (reject) ┘             └→ Superseded → Archived
```

### Value Objects

#### PolicyType
- AccessControl, DataGovernance, Compliance
- Operational, Security, ApprovalWorkflow, Custom

#### PolicyScope
```rust
enum PolicyScope {
    Global,
    Organization(Uuid),
    Context(String),
    ResourceType(String),
    Entities(HashSet<Uuid>),
    Custom(HashMap<String, Value>),
}
```

### Components

#### RulesComponent
```rust
- rules: Vec<PolicyRule>
- engine_type: String
- version: String
```

#### ApprovalRequirementsComponent
```rust
- minimum_approvals: u32
- required_approvers: HashSet<Uuid>
- required_roles: HashSet<String>
- approval_timeout: Option<Duration>
- external_approvals: Vec<ExternalApprovalRequirement>
```

#### External Approval Support

Supports external verification methods:
- Yubikey authentication
- Biometric verification
- Two-factor authentication
- Custom verification systems

### Business Rules

- State transitions validated according to state machine
- External approvals tracked with metadata
- Approval requirements must be met before activation
- Superseded policies maintain reference to replacement
- Archived state is terminal

## Common Patterns

### Component Architecture

All aggregates use a component-based architecture:

```rust
pub trait Component: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn Component>;
    fn serialize(&self) -> Result<Vec<u8>, Error>;
    fn component_type(&self) -> &'static str;
}
```

Benefits:
- Runtime extensibility
- Type safety with downcasting
- Serialization support
- Metadata tracking

### Event Sourcing

All aggregates follow event sourcing patterns:
1. Commands validate business rules
2. Events record state changes
3. Apply methods update aggregate state
4. No direct mutations allowed

### Version Control

All aggregates track versions for:
- Optimistic concurrency control
- Conflict detection
- Audit trails
- Event correlation

## Testing Strategy

Each aggregate includes comprehensive tests:

| Aggregate | Test Count | Coverage Areas |
|-----------|------------|----------------|
| Person | 15+ | Components, views, projections |
| Organization | 5+ | Hierarchy, members, locations |
| Agent | 5+ | State machine, permissions |
| Location | 4+ | Validation, calculations |
| Policy | 8+ | Workflows, approvals, states |

## Best Practices

1. **Aggregate Boundaries**: Keep aggregates small and focused
2. **Event Design**: Events record what happened, not how
3. **Command Validation**: Validate all invariants before events
4. **Component Design**: Make components immutable
5. **State Machines**: Use explicit states and transitions
6. **Value Objects**: Validate on construction, ensure immutability

## Future Enhancements

### Document Aggregate (Planned)
- Version control for files
- Content addressing with CIDs
- Access control integration
- Metadata and tagging

### Workflow Aggregate (Planned)
- Process orchestration
- Task management
- State machine composition
- Saga pattern support

## Summary

The CIM Domain aggregates provide a robust foundation for building event-driven, domain-focused applications. Each aggregate:

- Maintains clear boundaries
- Enforces business rules
- Supports extensibility through components
- Integrates with the event sourcing infrastructure
- Provides comprehensive test coverage

For implementation examples, see the [examples directory](../../examples/).