# CIM Domain Core Entities Implementation Plan

## Overview

This plan outlines the implementation of the five core domain entities identified for any Composable Information Machine (CIM). These entities form the minimal, essential foundation for modeling information flows and organizational structures.

## Core Domain Entities

### 1. People (Human Actors)
- **Purpose**: Represent human actors with identity, authentication, and decision-making capabilities
- **Key Components**: PersonId, authentication credentials, profile data, audit trail
- **Dependencies**: None (foundational entity)

### 2. Agents (Automated Actors)
- **Purpose**: Automated actors that execute tasks within bounded capabilities
- **Key Components**: AgentId, capabilities, owner relationship, execution logs
- **Dependencies**: People (agents must have owners)

### 3. Organizations (Collective Entities)
- **Purpose**: Group people and agents with hierarchical structures
- **Key Components**: OrganizationId, membership roster, hierarchy, roles
- **Dependencies**: People, Agents (as members)

### 4. Locations (Spatial Context)
- **Purpose**: Physical or logical spaces for activities and resources
- **Key Components**: LocationId, spatial data, access rules, resource tracking
- **Dependencies**: None (but relates to all entities)

### 5. Policies (Governance Rules)
- **Purpose**: Rules and constraints that govern system behavior
- **Key Components**: PolicyId, rules engine, enforcement mechanisms, audit
- **Dependencies**: All entities (policies apply to everything)

## Implementation Phases

### Phase 1: Foundation (Current)
**Goal**: Implement People entity with full identity management

**Tasks**:
1. Create person module structure
   ```rust
   // cim-domain/src/person/mod.rs
   pub mod aggregate;
   pub mod commands;
   pub mod events;
   pub mod value_objects;
   ```

2. Define PersonId value object
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Hash)]
   pub struct PersonId(Uuid);
   ```

3. Create Person aggregate
   ```rust
   pub struct Person {
       id: PersonId,
       name: PersonName,
       email: Email,
       created_at: DateTime<Utc>,
       updated_at: DateTime<Utc>,
   }
   ```

4. Implement core events
   - PersonRegistered
   - PersonAuthenticated
   - PersonProfileUpdated

5. Add command handlers
   - RegisterPerson
   - AuthenticatePerson
   - UpdatePersonProfile

6. Write comprehensive tests

### Phase 2: Automation
**Goal**: Add Agent entity with capability management

**Tasks**:
1. Define agent capabilities enum
2. Create Agent aggregate with owner relationship
3. Implement capability assignment and validation
4. Add execution tracking
5. Create agent-specific events and commands

### Phase 3: Organization
**Goal**: Implement Organizations with membership and hierarchy

**Tasks**:
1. Create organization structure with roles
2. Implement membership management
3. Add hierarchical relationships
4. Define organization-specific policies
5. Create events for all state changes

### Phase 4: Spatial Context
**Goal**: Add Location entity with access control

**Tasks**:
1. Define location types (physical, logical, virtual)
2. Implement spatial relationships
3. Add access control mechanisms
4. Create resource tracking
5. Integrate with other entities

### Phase 5: Governance
**Goal**: Implement Policy framework

**Tasks**:
1. Create policy definition language
2. Implement rule evaluation engine
3. Add policy binding to entities
4. Create compliance monitoring
5. Implement policy versioning

## Technical Decisions

### 1. Identity Generation
- Use UUID v7 for time-ordered unique identifiers
- Implement Display trait for human-readable formats
- Add validation in constructors

### 2. Event Sourcing
- All state changes through events
- Events are immutable and append-only
- Include causation and correlation IDs

### 3. Value Objects
- Immutable by design
- Self-validating constructors
- Rich domain behavior

### 4. Testing Strategy
- Unit tests for each value object
- Integration tests for aggregates
- Acceptance tests for user stories
- Property-based tests for invariants

## File Structure

```
cim-domain/
├── src/
│   ├── lib.rs
│   ├── person/
│   │   ├── mod.rs
│   │   ├── aggregate.rs
│   │   ├── commands.rs
│   │   ├── events.rs
│   │   └── value_objects.rs
│   ├── agent/
│   │   └── ... (similar structure)
│   ├── organization/
│   │   └── ... (similar structure)
│   ├── location/
│   │   └── ... (similar structure)
│   ├── policy/
│   │   └── ... (similar structure)
│   └── relationships/
│       ├── mod.rs
│       ├── affiliation.rs
│       ├── delegation.rs
│       └── binding.rs
├── tests/
│   ├── person_tests.rs
│   ├── agent_tests.rs
│   ├── organization_tests.rs
│   ├── location_tests.rs
│   ├── policy_tests.rs
│   └── integration_tests.rs
└── doc/
    ├── progress/
    │   └── progress.json
    ├── qa/
    │   └── cim-domain-user-stories.md
    └── plan/
        └── core-entities-implementation.md
```

## Success Criteria

1. **Completeness**: All 5 core entities implemented with tests
2. **Quality**: 95%+ test coverage
3. **Performance**: Sub-millisecond operations
4. **Documentation**: Full rustdoc with examples
5. **Integration**: Clean API for downstream usage

## Next Steps

1. Begin Phase 1 implementation
2. Create person module structure
3. Implement PersonId value object
4. Write first failing test
5. Make test pass with minimal code

This plan provides a clear roadmap for implementing the core CIM domain entities. Each phase builds on the previous one, ensuring a solid foundation for the entire system.
