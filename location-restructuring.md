# Location Domain Model Restructuring

## Overview

This document summarizes the restructuring of Location from a simple string to a proper Domain-Driven Design aggregate with rich value objects.

## Previous State

Location was represented as:
- `location: Option<String>` in Person commands/events
- Simple string in DefineLocation command
- No validation or structure

## New Domain Model

### Location Aggregate

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

### LocationType Enum

```rust
pub enum LocationType {
    Physical,    // Real-world locations
    Virtual,     // Online/digital spaces
    Logical,     // Organizational boundaries
    Hybrid,      // Mixed physical/virtual
}
```

### Value Objects

#### Address
```rust
pub struct Address {
    pub street1: String,
    pub street2: Option<String>,
    pub locality: String,      // City
    pub region: String,         // State/Province
    pub country: String,
    pub postal_code: String,
}
```

**Invariants:**
- All required fields must be non-empty
- Future: Country-specific postal code validation
- Future: Region validation based on country

#### GeoCoordinates
```rust
pub struct GeoCoordinates {
    pub latitude: f64,          // -90 to 90
    pub longitude: f64,         // -180 to 180
    pub altitude: Option<f64>,  // meters
    pub coordinate_system: String, // default: WGS84
}
```

**Features:**
- Range validation for lat/lon
- Distance calculation using Haversine formula
- Support for different coordinate systems

#### VirtualLocation
```rust
pub struct VirtualLocation {
    pub platform: String,
    pub platform_id: String,
    pub url: Option<String>,
    pub platform_data: HashMap<String, String>,
}
```

## Factory Methods

```rust
// Create physical location with address
Location::new_physical(id, name, address)

// Create virtual location
Location::new_virtual(id, name, virtual_location)

// Create location from coordinates only
Location::new_from_coordinates(id, name, coordinates)
```

## Business Rules

1. **Type Consistency**: Virtual locations cannot have physical addresses or coordinates
2. **Hierarchy**: Locations can have parent locations (no self-reference)
3. **Validation**: All value objects validate their invariants at creation
4. **Immutability**: Value objects are immutable - changes create new instances

## Command/Event Updates

### Commands
- `DefineLocation` now includes all location types and value objects
- `RegisterPerson` uses `location_id: Option<Uuid>` instead of string
- `UpdatePersonProfile` uses `location_id: Option<Uuid>`

### Events
- `LocationDefined` includes full location structure
- `PersonRegistered` references location by ID

## Benefits

1. **Type Safety**: Compile-time guarantees for location data
2. **Validation**: Business rules enforced at domain level
3. **Flexibility**: Supports multiple location representations
4. **Extensibility**: Easy to add new location types or validations
5. **Rich Domain Model**: Locations are first-class domain concepts

## Migration Notes

- Person entities now reference locations by ID
- Location must be created before being assigned to a person
- Old string-based locations need migration to proper Location aggregates

## Future Enhancements

1. **Country-Specific Validation**
   - Postal code formats
   - Valid regions per country
   - Address formatting rules

2. **Geocoding Integration**
   - Convert addresses to coordinates
   - Reverse geocoding support

3. **Location Services**
   - Find nearby locations
   - Location hierarchy navigation
   - Boundary detection (is point in region?)

4. **Additional Location Types**
   - Mobile locations (GPS tracking)
   - Relative locations (10m north of X)
   - Area/Region definitions

## Testing

Added 4 comprehensive tests:
- Address validation with invariants
- GeoCoordinates range validation
- Location aggregate creation
- Distance calculations

All tests maintain domain isolation and follow TDD principles.
