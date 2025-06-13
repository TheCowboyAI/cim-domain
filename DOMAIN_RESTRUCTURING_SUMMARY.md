# Domain Restructuring Summary

## Overview

This document summarizes the significant domain model improvements made to the cim-domain crate, transforming simple string-based representations into rich, component-based aggregates that properly model business concepts.

## Location Domain Restructuring

### Before
```rust
pub struct DefineLocation {
    pub location_id: Uuid,
    pub name: String,
    pub location: String,  // Simple string representation
}
```

### After
```rust
pub struct Location {
    entity: Entity<LocationMarker>,
    version: u64,
    pub name: String,
    pub location_type: LocationType,
    pub address: Option<Address>,
    pub coordinates: Option<GeoCoordinates>,
    pub virtual_location: Option<VirtualLocation>,
    pub parent_id: Option<EntityId<LocationMarker>>,
    pub metadata: HashMap<String, String>,
}
```

### Key Improvements

1. **Location as Aggregate**
   - Proper entity with identity and version control
   - Supports hierarchical locations (parent/child)
   - Rich metadata support

2. **Address Value Object**
   ```rust
   pub struct Address {
       pub street1: String,
       pub street2: Option<String>,
       pub locality: String,      // city/town
       pub region: String,        // state/province
       pub country: String,
       pub postal_code: String,
   }
   ```
   - All required fields validated
   - Immutable once created
   - Future: country-specific validation rules

3. **GeoCoordinates Value Object**
   ```rust
   pub struct GeoCoordinates {
       pub latitude: f64,         // -90 to 90
       pub longitude: f64,        // -180 to 180
       pub altitude: Option<f64>,
       pub coordinate_system: String,  // Default: WGS84
   }
   ```
   - Range validation on creation
   - Distance calculations using Haversine formula
   - Support for different coordinate systems

4. **LocationType Enum**
   - Physical: Real-world locations
   - Virtual: Online/digital spaces
   - Logical: Organizational boundaries
   - Hybrid: Mixed physical/virtual

## Person Domain Restructuring

### Before
```rust
pub struct RegisterPerson {
    pub person_id: Uuid,
    pub name: String,
    pub email: String,
    pub location: Option<String>,
}
```

### After
```rust
pub struct Person {
    entity: Entity<PersonMarker>,
    version: u64,
    components: ComponentStorage,
    component_metadata: HashMap<String, ComponentMetadata>,
}
```

### Key Improvements

1. **Component-Based Architecture**
   - Person is a container for components
   - Components can be added/removed dynamically
   - Each component has metadata (who added, when, why)

2. **Core Components**

   **IdentityComponent**
   ```rust
   pub struct IdentityComponent {
       pub legal_name: String,
       pub preferred_name: Option<String>,
       pub date_of_birth: Option<chrono::NaiveDate>,
       pub government_id: Option<String>,
   }
   ```

   **ContactComponent**
   ```rust
   pub struct ContactComponent {
       pub emails: Vec<EmailAddress>,
       pub phones: Vec<PhoneNumber>,
       pub addresses: Vec<Uuid>,  // References to Location aggregates
   }
   ```

   **EmploymentComponent**
   ```rust
   pub struct EmploymentComponent {
       pub organization_id: Uuid,
       pub employee_id: String,
       pub title: String,
       pub department: Option<String>,
       pub manager_id: Option<Uuid>,
       pub status: String,
       pub start_date: chrono::NaiveDate,
       pub end_date: Option<chrono::NaiveDate>,
   }
   ```

3. **View Projections**

   **EmployeeView**
   - Combines identity, contact, employment, position, and skills
   - Created from Person aggregate when needed
   - Validates required components exist

   **LdapProjection**
   - Maps Person components to LDAP attributes
   - Generates DN from base DN and preferred name
   - Extracts mail and telephoneNumber arrays

4. **External System Support**
   ```rust
   pub struct ExternalIdentifiersComponent {
       pub ldap_dn: Option<String>,
       pub ad_sid: Option<String>,
       pub oauth_subjects: HashMap<String, String>,
       pub external_ids: HashMap<String, String>,
   }
   ```

## Benefits of Restructuring

### 1. **Flexibility**
- New person types (Customer, Contractor) are just different component combinations
- New location types can be added without changing core structure
- Components can evolve independently

### 2. **Type Safety**
- Compiler enforces valid lat/lon ranges
- Required fields can't be null
- Component types are checked at compile time

### 3. **Business Alignment**
- Models match real-world concepts
- Supports complex scenarios (multi-role persons, hierarchical locations)
- Clear separation of identity vs. contact vs. employment

### 4. **Integration Ready**
- LDAP projection for directory services
- OAuth identifiers for SSO
- Location references instead of embedded strings

### 5. **Event Sourcing Compatible**
- Components are immutable (replaced, not mutated)
- Version tracking for optimistic concurrency
- Clear aggregate boundaries

## Migration Path

### For Existing Code

1. **Location Migration**
   ```rust
   // Old
   let location = "123 Main St, City, State";

   // New
   let address = Address::new(
       "123 Main St",
       None,
       "City",
       "State",
       "Country",
       "12345"
   )?;
   let location = Location::new_physical(id, "Office", address);
   ```

2. **Person Migration**
   ```rust
   // Old
   let person = RegisterPerson {
       person_id,
       name: "John Doe",
       email: "john@example.com",
       location: Some("New York"),
   };

   // New
   let identity = IdentityComponent {
       legal_name: "John Doe",
       preferred_name: None,
       date_of_birth: None,
       government_id: None,
   };
   let mut person = Person::new(person_id, identity);

   let contact = ContactComponent {
       emails: vec![EmailAddress {
           email: "john@example.com",
           email_type: "personal",
           is_primary: true,
           is_verified: false,
       }],
       phones: vec![],
       addresses: vec![location_id],
   };
   person.add_component(contact, "system", None)?;
   ```

## Future Enhancements

1. **Location**
   - Geocoding service integration
   - Address validation by country
   - Indoor positioning (floor, room, desk)
   - Geofencing capabilities

2. **Person**
   - BiometricComponent for authentication
   - PreferencesComponent for personalization
   - HealthComponent for medical systems
   - FinancialComponent for banking

3. **Cross-Aggregate**
   - Person location history tracking
   - Organization location hierarchies
   - Location-based access control

## Conclusion

The domain restructuring transforms simple data containers into rich domain models that:
- Properly represent business concepts
- Enforce invariants at the type level
- Support complex real-world scenarios
- Enable clean integration with external systems
- Maintain clear aggregate boundaries

This foundation enables building sophisticated business applications while maintaining code clarity and type safety.
