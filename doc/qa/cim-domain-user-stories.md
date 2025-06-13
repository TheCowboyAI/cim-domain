# CIM Domain User Stories and Acceptance Tests

## Overview

This document defines user stories and acceptance tests for the core domain entities that form the foundation of any Composable Information Machine (CIM). These entities represent the minimal, essential concepts needed to model information flows and organizational structures.

## Event-Driven Architecture Principles

**Important**: In CIM's event-driven architecture:
- **Commands** produce an acknowledgment and trigger event streams
- **Queries** produce an acknowledgment and trigger event streams
- **Results** are delivered through subscribed event streams
- **No synchronous returns** - all interactions are asynchronous via events

## Core Domain Entities

### 1. People (Human Actors)

**Definition**: People represent human actors in the system who can make decisions, own resources, and interact with the CIM.

#### User Stories

**US-P1: Person Registration**
```
As a system administrator
I want to register a new person in the CIM
So that they can be identified and participate in workflows
```

**Acceptance Tests:**
- GIVEN a valid person registration command
- WHEN the command is submitted to the system
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers to the person event stream receive a PersonRegistered event
- AND the event contains person ID, timestamp, and registration details
- AND subsequent queries can retrieve the person via event streams

**US-P2: Person Authentication**
```
As a registered person
I want to authenticate my identity
So that I can access my authorized resources and capabilities
```

**Acceptance Tests:**
- GIVEN a registered person with credentials
- WHEN they submit an authentication command
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive either PersonAuthenticated or AuthenticationFailed event
- AND PersonAuthenticated event contains session token and expiry
- AND the person's authentication history is updated in the event stream

**US-P3: Person Profile Management**
```
As a registered person
I want to update my profile information
So that my information remains current and accurate
```

**Acceptance Tests:**
- GIVEN an authenticated person
- WHEN they submit a profile update command
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive a PersonProfileUpdated event
- AND the event contains the changed fields and previous values
- AND an audit trail event is emitted to the audit stream

### 2. Agents (Automated Actors)

**Definition**: Agents are automated actors that can execute tasks, make decisions within defined parameters, and interact with other system components on behalf of people or organizations.

#### User Stories

**US-A1: Agent Creation**
```
As a person with agent creation privileges
I want to create an agent with specific capabilities
So that it can perform automated tasks on my behalf
```

**Acceptance Tests:**
- GIVEN a person with agent creation privileges
- WHEN they submit a create agent command
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive an AgentCreated event
- AND the event contains agent ID, owner ID, and capability manifest
- AND a separate AgentCapabilitiesDefined event details the granted capabilities
- AND the agent becomes available for task assignment via events

**US-A2: Agent Capability Assignment**
```
As an agent owner
I want to assign specific capabilities to my agent
So that it can perform authorized actions within defined boundaries
```

**Acceptance Tests:**
- GIVEN an existing agent and its owner
- WHEN a capability assignment command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive an AgentCapabilitiesUpdated event
- AND if capabilities violate policies, a CapabilityAssignmentRejected event is emitted
- AND approved capabilities are reflected in subsequent AgentCapabilityQueried events

**US-A3: Agent Execution Monitoring**
```
As an agent owner
I want to monitor my agent's activities
So that I can ensure it operates within expected parameters
```

**Acceptance Tests:**
- GIVEN an active agent performing tasks
- WHEN the agent executes any action
- THEN an AgentActionInitiated event is emitted to the monitoring stream
- AND an AgentActionCompleted or AgentActionFailed event follows
- AND resource usage events are emitted to the metrics stream
- AND anomaly detection triggers AgentAnomalyDetected events when thresholds exceeded

### 3. Organizations (Collective Entities)

**Definition**: Organizations represent collective entities that group people and agents, own resources, and have hierarchical structures.

#### User Stories

**US-O1: Organization Formation**
```
As a person with organization creation privileges
I want to form a new organization
So that we can coordinate collective activities and resources
```

**Acceptance Tests:**
- GIVEN a person with organization creation privileges
- WHEN they submit an organization formation command
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive an OrganizationFormed event
- AND the event contains organization ID, founder ID, and founding timestamp
- AND an OrganizationAdministratorAssigned event assigns the founder as admin
- AND the organization becomes available for membership via event streams

**US-O2: Organization Membership**
```
As an organization administrator
I want to add and remove members (people and agents)
So that we can manage who participates in our organization
```

**Acceptance Tests:**
- GIVEN an organization with an administrator
- WHEN a membership change command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive MemberAdded or MemberRemoved events
- AND access rights changes trigger MemberAccessRightsUpdated events
- AND affected parties receive notifications via MembershipNotification events
- AND the organization roster is updated through event projection

**US-O3: Organization Hierarchy**
```
As an organization administrator
I want to create sub-organizations and departments
So that we can model our organizational structure
```

**Acceptance Tests:**
- GIVEN a parent organization
- WHEN a sub-organization creation command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive an OrganizationStructureUpdated event
- AND if hierarchy creates cycles, a HierarchyValidationFailed event is emitted
- AND valid structures trigger InheritanceRulesApplied events
- AND reporting relationships are established via ReportingLineCreated events

### 4. Locations (Spatial Context)

**Definition**: Locations represent physical or logical spaces where activities occur, resources exist, and access can be controlled.

#### User Stories

**US-L1: Location Definition**
```
As a system administrator
I want to define locations in the system
So that we can model where activities occur and resources exist
```

**Acceptance Tests:**
- GIVEN location information (name, type, coordinates/address)
- WHEN a location definition command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive a LocationDefined event
- AND the event contains location ID and spatial data
- AND spatial relationships trigger SpatialRelationshipEstablished events
- AND access zones are defined via AccessZoneCreated events

**US-L2: Location Access Control**
```
As a location manager
I want to control who can access specific locations
So that we can ensure security and proper resource allocation
```

**Acceptance Tests:**
- GIVEN a defined location with a manager
- WHEN access rule commands are submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive LocationAccessRulesUpdated events
- AND access attempts trigger AccessAttempted events
- AND granted access emits AccessGranted events with duration
- AND denied access emits AccessDenied events with reason
- AND violations trigger SecurityAlertRaised events

**US-L3: Location Resource Tracking**
```
As a location manager
I want to track resources at my location
So that we can manage inventory and utilization
```

**Acceptance Tests:**
- GIVEN a location with trackable resources
- WHEN resource movement commands are submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive ResourceEntered or ResourceExited events
- AND inventory changes trigger InventoryLevelUpdated events
- AND capacity violations emit CapacityExceeded events
- AND movement history is maintained through event stream

### 5. Policies (Governance Rules)

**Definition**: Policies represent rules, constraints, and governance mechanisms that control behavior and ensure compliance across the system.

#### User Stories

**US-PO1: Policy Definition**
```
As a governance administrator
I want to define policies that govern system behavior
So that we can ensure compliance and consistent operations
```

**Acceptance Tests:**
- GIVEN policy parameters (scope, rules, enforcement)
- WHEN a policy definition command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive a PolicyDefined event
- AND the event contains policy ID and rule specifications
- AND validation results in PolicyValidated or PolicyValidationFailed events
- AND affected entities receive PolicyActivationNotification events

**US-PO2: Policy Enforcement**
```
As a system component
I want policies to be automatically enforced
So that all actions comply with governance rules
```

**Acceptance Tests:**
- GIVEN an active policy and a proposed action
- WHEN the action is evaluated against the policy
- THEN a PolicyEvaluationRequested event is emitted
- AND subscribers receive PolicyEvaluationCompleted event
- AND the event contains allow/deny decision and reasoning
- AND violations trigger PolicyViolationDetected events
- AND enforcement actions emit PolicyEnforcementExecuted events

**US-PO3: Policy Evolution**
```
As a governance administrator
I want to update policies based on changing requirements
So that governance remains relevant and effective
```

**Acceptance Tests:**
- GIVEN an existing policy requiring update
- WHEN a policy update command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive a PolicyUpdated event with version info
- AND previous version emits PolicyVersionArchived event
- AND affected entities receive PolicyUpdateNotification events
- AND grace period triggers PolicyGracePeriodStarted event

## Cross-Entity Interactions

### Relationship User Stories

**US-R1: Person-Organization Affiliation**
```
As a person
I want to affiliate with organizations
So that I can participate in collective activities
```

**Acceptance Tests:**
- GIVEN a person and an organization
- WHEN an affiliation command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive an AffiliationCreated event
- AND role assignment triggers RoleAssigned events
- AND access rights emit AccessRightsGranted events
- AND both entities receive relationship events in their streams

**US-R2: Agent-Person Delegation**
```
As a person
I want to delegate authority to my agents
So that they can act on my behalf
```

**Acceptance Tests:**
- GIVEN a person and their agent
- WHEN a delegation command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive a DelegationCreated event
- AND the event contains scope and limitations
- AND agent actions emit ActionPerformedOnBehalfOf events
- AND revocation triggers DelegationRevoked event

**US-R3: Policy-Entity Binding**
```
As a policy administrator
I want to bind policies to specific entities
So that governance is properly scoped
```

**Acceptance Tests:**
- GIVEN a policy and target entities
- WHEN a binding command is submitted
- THEN a CommandAccepted event is emitted with correlation ID
- AND subscribers receive PolicyBound events for each entity
- AND inheritance triggers PolicyInheritanceApplied events
- AND conflicts emit PolicyConflictDetected events
- AND resolution triggers ConflictResolved events

## Event Stream Patterns

### Command Pattern
```rust
// Command submission
let correlation_id = CorrelationId::new();
let command = RegisterPerson {
    correlation_id,
    name: "Alice Smith",
    email: "alice@example.com",
};

// Returns only acknowledgment
let ack = system.submit_command(command).await?;
assert_eq!(ack.status, CommandStatus::Accepted);
assert_eq!(ack.correlation_id, correlation_id);

// Results come through event subscription
let mut event_stream = system.subscribe_to("person.events").await?;
while let Some(event) = event_stream.next().await {
    match event {
        Event::PersonRegistered { correlation_id: cid, person_id, .. } => {
            if cid == correlation_id {
                // Handle successful registration
            }
        }
        Event::RegistrationFailed { correlation_id: cid, reason } => {
            if cid == correlation_id {
                // Handle failure
            }
        }
        _ => {}
    }
}
```

### Query Pattern
```rust
// Query submission
let correlation_id = CorrelationId::new();
let query = FindPersonById {
    correlation_id,
    person_id: PersonId::from("alice-123"),
};

// Returns only acknowledgment
let ack = system.submit_query(query).await?;
assert_eq!(ack.status, QueryStatus::Accepted);

// Results come through event subscription
let mut result_stream = system.subscribe_to("query.results").await?;
while let Some(event) = result_stream.next().await {
    match event {
        Event::QueryResult { correlation_id: cid, data } => {
            if cid == correlation_id {
                // Process query results
            }
        }
        Event::QueryFailed { correlation_id: cid, reason } => {
            if cid == correlation_id {
                // Handle query failure
            }
        }
        _ => {}
    }
}
```

## Domain Invariants

### Core Invariants to Maintain

1. **Identity Uniqueness**: Every entity must have a globally unique identifier
2. **Temporal Consistency**: All events must have timestamps and maintain causal ordering
3. **Authorization Integrity**: No action without proper authorization (verified through events)
4. **Audit Completeness**: Every state change must produce events
5. **Policy Compliance**: All actions must be evaluated against policies (async)
6. **Event Immutability**: Published events cannot be modified or deleted
7. **Correlation Tracking**: All commands/queries must have correlation IDs for result matching

## Implementation Priorities

### Phase 1: Foundation (Weeks 1-2)
- People entity with event-driven CRUD
- Command/Query acknowledgment pattern
- Event stream subscriptions
- Correlation ID tracking

### Phase 2: Automation (Weeks 3-4)
- Agent creation via events
- Capability assignment through event streams
- Execution monitoring events

### Phase 3: Organization (Weeks 5-6)
- Organization formation events
- Membership event streams
- Hierarchical event propagation

### Phase 4: Spatial Context (Weeks 7-8)
- Location definition events
- Access control event streams
- Resource tracking via events

### Phase 5: Governance (Weeks 9-10)
- Policy definition events
- Asynchronous policy evaluation
- Compliance event streams

## Success Metrics

1. **Coverage**: All user stories have event-driven acceptance tests
2. **Performance**: Event publication <10ms, subscription delivery <50ms
3. **Scalability**: Support 100,000+ events/second
4. **Reliability**: No event loss, exactly-once delivery semantics
5. **Auditability**: Complete event history with correlation tracking

## Fitness Functions

```rust
// Example fitness function for event-driven commands
#[test]
fn test_command_returns_only_acknowledgment() {
    let mut system = CIMSystem::new();
    let correlation_id = CorrelationId::new();
    let command = RegisterPerson {
        correlation_id,
        name: "Alice",
        email: "alice@example.com",
    };

    // Command returns only acknowledgment
    let ack = system.submit_command(command).await.unwrap();
    assert_eq!(ack.status, CommandStatus::Accepted);
    assert_eq!(ack.correlation_id, correlation_id);

    // No direct result - would panic if tried
    // let person = ack.result; // This field doesn't exist!
}

// Example fitness function for event stream results
#[test]
fn test_results_via_event_stream() {
    let mut system = CIMSystem::new();
    let correlation_id = CorrelationId::new();

    // Subscribe before submitting command
    let mut events = system.subscribe_to("person.events").await.unwrap();

    // Submit command
    let command = RegisterPerson { correlation_id, /* ... */ };
    system.submit_command(command).await.unwrap();

    // Results come through events
    let event = events.next().await.unwrap();
    match event {
        Event::PersonRegistered { correlation_id: cid, person_id, .. } => {
            assert_eq!(cid, correlation_id);
            assert!(!person_id.is_empty());
        }
        _ => panic!("Expected PersonRegistered event"),
    }
}
```

## Next Steps

1. Review and refine event-driven patterns with domain experts
2. Implement event stream infrastructure in cim-domain crate
3. Create comprehensive event-driven test suite
4. Build NATS-based event streaming
5. Implement correlation tracking system

This document serves as the foundation for building the core CIM domain with proper event-driven architecture. All interactions are asynchronous through event streams, ensuring scalability and loose coupling.
