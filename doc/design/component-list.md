# CIM Domain Component List

## Current Components by Module

### component.rs
- `Component` (trait) - Base trait for attachable components
- `ComponentStorage` - Storage for type-erased components

### entity.rs
- `Entity` (trait) - Types with identity
- `EntityId` - Unique entity identifier
- `AggregateRoot` (trait) - Consistency boundary marker
- Marker types:
  - `GraphMarker`
  - `AggregateMarker`
  - `BoundedContextMarker`
  - `EntityMarker`
  - `ValueObjectMarker`
  - `ServiceMarker`
  - `EventMarker`
  - `CommandMarker`
  - `QueryMarker`

### identifiers.rs
- `NodeId` - Graph node identifier
- `EdgeId` - Graph edge identifier
- `GraphId` - Graph identifier

### node_types.rs
- `NodeType` (enum) - Types of nodes in graphs

### relationship_types.rs
- `RelationshipType` (enum) - Types of relationships/edges

### context_types.rs
- `ContextType` (enum) - Bounded context types
- `SubdomainType` (enum) - Subdomain classifications
- `ServiceType` (enum) - Service types

### composition_types.rs
- `CompositionType` (enum) - How components compose
- `DomainCompositionType` (enum) - Domain-specific composition

### cqrs.rs
- `Command` (trait) - Command interface
- `Query` (trait) - Query interface
- `CommandId` - Command identifier
- `QueryId` - Query identifier
- `EventId` - Event identifier
- `IdType` (enum) - Uuid or Cid
- `CorrelationId` - Correlation tracking
- `CausationId` - Causation tracking
- `CommandEnvelope` - Command with metadata
- `QueryEnvelope` - Query with metadata
- `CommandHandler` (trait) - Processes commands
- `QueryHandler` (trait) - Processes queries
- `CommandStatus` (enum) - Accepted/Rejected
- `QueryStatus` (enum) - Accepted/Rejected
- `CommandAcknowledgment` - Command response
- `QueryAcknowledgment` - Query response
- `EventStreamSubscription` - For async results

### state_machine.rs
- `State` (trait) - State interface
- `StateTransitions` (trait) - Valid transitions
- `StateMachine<S>` - Generic state machine
- `StateTransition<S>` - Transition record
- `OrderState` (enum) - Example order states
- `PersonState` (enum) - Example person states

### subjects.rs
- `SubjectParts` - Parsed subject components
- `SubjectPattern` - Pattern with wildcards
- `SubjectPermissions` - Access control
- `PropagationScope` (enum) - Local/Cluster/Global
- `EventEnvelope` - Event with routing info
- `MessageTranslator` (trait) - Translation interface
- `SubjectParser` (trait) - Subject parsing

### events.rs
- `DomainEvent` (trait) - Base event interface
- `EventMetadata` - Event metadata
- `DomainEventEnvelope<T>` - Generic event wrapper
- Core entity events:
  - `PersonRegistered`
  - `OrganizationCreated`
  - `AgentDeployed`
  - `LocationDefined`
  - `PolicyEnacted`

### commands.rs
- Core entity commands:
  - `RegisterPerson`
  - `UpdatePersonProfile`
  - `CreateOrganization`
  - `AddOrganizationMember`
  - `DeployAgent`
  - `UpdateAgentCapabilities`
  - `DefineLocation`
  - `EnactPolicy`
  - `UpdatePolicyRules`

### bevy_bridge.rs
- `ComponentData` - Generic component representation
- `BevyCommand` (enum) - ECS commands
  - `SpawnEntity`
  - `UpdateEntity`
  - `DespawnEntity`
  - `CreateRelationship`
- `BevyEvent` (enum) - UI events
  - `EntitySelected`
  - `EntityMoved`
  - `EntityCreationRequested`
- `ComponentMapper` - Maps domain to ECS
- `NatsMessage` - NATS message wrapper
- `TranslationError` (enum) - Translation errors
- `NatsToBevyTranslator` - NATS to Bevy translation
- `BevyEventRouter` - Routes Bevy events to subjects

### errors.rs
- `DomainError` (enum) - Domain-specific errors
- `DomainResult<T>` - Result type alias

## Summary Statistics

- **Traits**: 11
- **Enums**: 15
- **Structs**: 25+
- **Core Entities**: 5 (Person, Organization, Agent, Location, Policy)
- **Event Types**: 5
- **Command Types**: 9
- **Total Public Types**: ~60+

## Module Dependencies

```
component.rs → (base, no deps)
entity.rs → component.rs
identifiers.rs → (base, uuid)
cqrs.rs → identifiers.rs, cid
state_machine.rs → (base)
subjects.rs → errors.rs
events.rs → subjects.rs, cqrs.rs
commands.rs → cqrs.rs
bevy_bridge.rs → events.rs, commands.rs, subjects.rs
errors.rs → (base, thiserror)
```
