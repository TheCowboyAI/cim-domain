<!-- Copyright 2025 Cowboy AI, LLC. -->

# Domain Model Analysis Report

## Summary

### Element Counts

| Type | Count |
|------|-------|
| Projection | 1 |
| ValueObject | 2 |
| Component | 2 |
| Aggregate | 2 |
| Command | 2 |
| Event | 2 |

### Relationship Summary

Total relationships: 8

| Relationship Type | Count |
|-------------------|-------|
| References | 1 |
| Projects | 1 |
| EmitsEvent | 2 |
| Uses | 2 |
| Contains | 2 |

## Detailed Element List

### Module: location

#### GeoCoordinates (ValueObject)

**Fields:**
- latitude: f64
- longitude: f64

**Methods:**
- distance_to

#### Location (Aggregate)

**Fields:**
- entity: Entity<LocationMarker>
- name: String
- location_type: LocationType
- address: Address?
- coordinates: GeoCoordinates?

**Methods:**
- new_physical
- new_virtual

**Implements:**
- AggregateRoot

#### Address (ValueObject)

**Fields:**
- street1: String
- locality: String
- region: String
- country: String

**Methods:**
- validate

### Module: person

#### ContactComponent (Component)

**Fields:**
- emails: EmailAddress[]
- addresses: Uuid[]

**Implements:**
- Component

#### EmployeeView (Projection)

**Fields:**
- person_id: EntityId<PersonMarker>
- identity: IdentityComponent
- employment: EmploymentComponent

**Methods:**
- from_person

#### Person (Aggregate)

**Fields:**
- entity: Entity<PersonMarker>
- components: ComponentStorage

**Methods:**
- add_component
- remove_component

**Implements:**
- AggregateRoot

#### IdentityComponent (Component)

**Fields:**
- legal_name: String
- preferred_name: String?

**Implements:**
- Component

### Module: commands

#### DefineLocation (Command)

**Fields:**
- location_id: Uuid
- address: Address?

**Implements:**
- Command

#### RegisterPerson (Command)

**Fields:**
- person_id: Uuid
- identity: IdentityComponent

**Implements:**
- Command

### Module: events

#### LocationDefined (Event)

**Fields:**
- location_id: Uuid
- address: Address?

**Implements:**
- DomainEvent

#### PersonRegistered (Event)

**Fields:**
- person_id: Uuid
- identity: IdentityComponent

**Implements:**
- DomainEvent


## Recommendations

Based on the analysis, consider:

1. **Missing Aggregates**: Organization, Agent, Policy
2. **Missing Value Objects**: EmailAddress, PhoneNumber are referenced but not fully defined
3. **Missing Events**: Many commands don't have corresponding events
4. **Missing Handlers**: No command handlers or event handlers are defined
5. **Graph-specific types**: No graph-related aggregates (Graph, Node, Edge) are defined
