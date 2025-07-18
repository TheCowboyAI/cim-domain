<!-- Copyright 2025 Cowboy AI, LLC. -->

# CIM Domain Component System

## Overview

The CIM Domain implements a sophisticated component system that enables dynamic composition, type-safe entity management, and seamless integration between domain logic and external systems. This document describes the component architecture, available components, and usage patterns.

## Component Architecture

### Core Design

The component system is built on three foundational principles:

1. **Type Erasure with Safety** - Components can be stored generically while maintaining type safety through downcasting
2. **Dynamic Composition** - Entities can have components attached or removed at runtime
3. **Domain Isolation** - Components are domain-specific with clear boundaries

### Base Component Trait

```rust
pub trait Component: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn Component>;
    fn serialize(&self) -> Result<Vec<u8>, Error>;
    fn component_type(&self) -> &'static str;
}
```

This trait provides:
- **Type erasure** through `Any`
- **Thread safety** with `Send + Sync`
- **Cloning** for immutable updates
- **Serialization** for persistence
- **Type identification** for runtime inspection

### Component Storage

```rust
pub struct ComponentStorage {
    components: HashMap<TypeId, Box<dyn Component>>,
    metadata: HashMap<TypeId, ComponentMetadata>,
}

pub struct ComponentMetadata {
    pub added_at: DateTime<Utc>,
    pub added_by: Option<Uuid>,
    pub tags: HashSet<String>,
    pub version: u64,
}
```

Key features:
- Type-indexed storage using `TypeId`
- Metadata tracking for audit trails
- Version control for optimistic concurrency
- Tag-based component queries

## Component Categories

### 1. Identity Components

Components that establish entity identity and authentication:

```rust
pub struct IdentityComponent {
    pub legal_name: String,
    pub preferred_name: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
    pub government_id: Option<String>,
}

pub struct ExternalIdentifiersComponent {
    pub ldap_dn: Option<String>,
    pub active_directory_sid: Option<String>,
    pub oauth_subjects: Vec<OAuthSubject>,
    pub external_ids: HashMap<String, String>,
}
```

### 2. Contact Components

Components for communication and location:

```rust
pub struct ContactComponent {
    pub emails: Vec<EmailAddress>,
    pub phones: Vec<PhoneNumber>,
    pub addresses: Vec<Uuid>, // Location references
    pub preferred_contact_method: ContactMethod,
}
```

### 3. Organizational Components

Components for organizational relationships:

```rust
pub struct EmploymentComponent {
    pub organization_id: Uuid,
    pub employee_id: String,
    pub title: String,
    pub department: Option<String>,
    pub manager_id: Option<Uuid>,
    pub status: EmploymentStatus,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
}

pub struct OrganizationMetadata {
    pub industry: Option<String>,
    pub size: Option<OrganizationSize>,
    pub website: Option<Url>,
    pub founded_date: Option<NaiveDate>,
}
```

### 4. Capability Components

Components that define what entities can do:

```rust
pub struct CapabilitiesComponent {
    pub capabilities: HashMap<String, CapabilityMetadata>,
    pub version: String,
}

pub struct PermissionsComponent {
    pub granted: HashSet<String>,
    pub denied: HashSet<String>,
    pub roles: HashSet<String>,
}

pub struct ToolAccessComponent {
    pub tools: HashMap<String, ToolDefinition>,
    pub usage_stats: HashMap<String, ToolUsageStats>,
}
```

### 5. Policy Components

Components for governance and rules:

```rust
pub struct RulesComponent {
    pub rules: Vec<PolicyRule>,
    pub engine_type: String,
    pub version: String,
}

pub struct ApprovalRequirementsComponent {
    pub minimum_approvals: u32,
    pub required_approvers: HashSet<Uuid>,
    pub required_roles: HashSet<String>,
    pub approval_timeout: Option<Duration>,
    pub external_approvals: Vec<ExternalApprovalRequirement>,
}
```

### 6. Configuration Components

Components for runtime configuration:

```rust
pub struct ConfigurationComponent {
    pub config_data: HashMap<String, Value>,
    pub schema_version: String,
    pub last_updated: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

pub struct AgentMetadata {
    pub name: String,
    pub description: Option<String>,
    pub tags: HashSet<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
}
```

## Component Patterns

### Adding Components

```rust
impl Person {
    pub fn add_identity(&mut self, identity: IdentityComponent) -> Result<(), DomainError> {
        // Validate component
        identity.validate()?;
        
        // Add with metadata
        self.components.add_component(identity);
        
        // Emit event
        self.pending_events.push(PersonEvent::ComponentAdded {
            person_id: self.entity.id(),
            component_type: "IdentityComponent".to_string(),
            added_at: Utc::now(),
        });
        
        Ok(())
    }
}
```

### Querying Components

```rust
impl Person {
    pub fn get_identity(&self) -> Option<&IdentityComponent> {
        self.components.get_component::<IdentityComponent>()
    }
    
    pub fn has_component<C: Component + 'static>(&self) -> bool {
        self.components.has::<C>()
    }
    
    pub fn list_components(&self) -> Vec<&'static str> {
        self.components.component_types()
    }
}
```

### Component Validation

```rust
pub trait ValidatedComponent: Component {
    type Error;
    
    fn validate(&self) -> Result<(), Self::Error>;
}

impl ValidatedComponent for EmailAddress {
    type Error = ValidationError;
    
    fn validate(&self) -> Result<(), Self::Error> {
        if self.0.contains('@') && self.0.len() > 3 {
            Ok(())
        } else {
            Err(ValidationError::InvalidEmail)
        }
    }
}
```

## Integration Components

### NATS Subject Components

```rust
pub struct NatsSubject {
    pub context: String,
    pub entity: String,
    pub event: String,
    pub version: String,
}

pub struct PropagationEnvelope {
    pub subject: NatsSubject,
    pub scope: PropagationScope,
    pub correlation_id: CorrelationId,
}
```

### Bevy ECS Bridge Components

```rust
pub struct BevyComponentAdded {
    pub entity_id: Entity,
    pub component_type: String,
    pub component_data: Vec<u8>,
}

pub struct BevyTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
```

## Component Lifecycle

### 1. Creation

Components are created with validation:

```rust
let identity = IdentityComponent::new(
    "John Doe",
    Some("John"),
    Some(NaiveDate::from_ymd(1990, 1, 1)),
    Some("123-45-6789")
)?;
```

### 2. Attachment

Components are attached to entities:

```rust
person.components.add_component(identity);
```

### 3. Modification

Components are immutable; modifications create new versions:

```rust
let updated_identity = identity.with_name("Jane Doe");
person.components.replace_component(updated_identity);
```

### 4. Removal

Components can be removed with tracking:

```rust
person.components.remove_component::<IdentityComponent>()?;
```

## Type Safety

### Marker Types

The system uses phantom types for compile-time safety:

```rust
pub struct PersonMarker;
pub struct OrganizationMarker;
pub struct AgentMarker;

pub struct EntityId<T> {
    id: Uuid,
    _phantom: PhantomData<T>,
}
```

### Type-Safe References

```rust
impl Person {
    pub fn manager_id(&self) -> Option<EntityId<PersonMarker>> {
        self.get_employment()
            .and_then(|e| e.manager_id)
            .map(|id| EntityId::new(id))
    }
}
```

## Performance Considerations

### Component Access

- **O(1)** lookup by `TypeId`
- **O(n)** iteration over all components
- **O(1)** existence checks

### Memory Usage

- Components stored as boxed trait objects
- Metadata overhead per component (~100 bytes)
- Lazy loading supported for large components

### Serialization

- Components serialize independently
- Supports partial serialization
- Binary and JSON formats supported

## Best Practices

### 1. Component Granularity

Keep components focused and cohesive:

```rust
// Good: Focused component
pub struct AddressComponent {
    pub street: String,
    pub city: String,
    pub postal_code: String,
}

// Bad: Kitchen sink component
pub struct PersonDataComponent {
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub employment: String,
    // ... many more fields
}
```

### 2. Validation

Always validate components on creation:

```rust
impl IdentityComponent {
    pub fn new(legal_name: String) -> Result<Self, ValidationError> {
        if legal_name.is_empty() {
            return Err(ValidationError::EmptyName);
        }
        
        Ok(Self {
            legal_name,
            // ... other fields
        })
    }
}
```

### 3. Immutability

Treat components as immutable:

```rust
impl ContactComponent {
    pub fn add_email(self, email: EmailAddress) -> Self {
        let mut emails = self.emails;
        emails.push(email);
        
        Self {
            emails,
            ..self
        }
    }
}
```

### 4. Event Emission

Always emit events for component changes:

```rust
self.pending_events.push(Event::ComponentAdded {
    entity_id: self.id(),
    component_type: std::any::type_name::<C>(),
    timestamp: Utc::now(),
});
```

## Future Enhancements

### Planned Features

1. **Component Queries** - DSL for complex component queries
2. **Component Inheritance** - Trait-based component hierarchies
3. **Component Migrations** - Versioned component evolution
4. **Component Indexing** - Fast queries by component properties
5. **Component Compression** - Automatic compression for large components

### Research Areas

1. **Graph-based Components** - Components as nodes in knowledge graphs
2. **ML-driven Components** - Components with embedded models
3. **Distributed Components** - Components spanning multiple nodes
4. **Reactive Components** - Components with built-in reactivity

## Summary

The CIM Domain component system provides:

- **Flexibility** through dynamic composition
- **Type Safety** through Rust's type system
- **Performance** through efficient storage
- **Extensibility** through trait-based design
- **Integration** through bridge components

This architecture enables building complex domain models that can evolve over time while maintaining type safety and performance.