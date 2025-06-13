//! Location aggregate and related value objects
//!
//! Location is an aggregate that can represent any identifiable place through
//! various means: addresses, geo-coordinates, virtual locations, etc.

use crate::{AggregateRoot, Entity, EntityId, DomainError, DomainResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Location aggregate - represents any identifiable place
#[derive(Debug, Clone)]
pub struct Location {
    /// Core entity data
    entity: Entity<LocationMarker>,

    /// Version for optimistic concurrency control
    version: u64,

    /// Human-readable name for the location
    pub name: String,

    /// Type of location (physical, virtual, logical, etc.)
    pub location_type: LocationType,

    /// Physical address if applicable
    pub address: Option<Address>,

    /// Geographic coordinates if applicable
    pub coordinates: Option<GeoCoordinates>,

    /// Virtual location details if applicable
    pub virtual_location: Option<VirtualLocation>,

    /// Parent location for hierarchical structures
    pub parent_id: Option<EntityId<LocationMarker>>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Marker type for Location entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocationMarker;

/// Types of locations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationType {
    /// Physical location with real-world presence
    Physical,
    /// Virtual location (e.g., online meeting room, game world)
    Virtual,
    /// Logical location (e.g., department, zone)
    Logical,
    /// Hybrid location with both physical and virtual aspects
    Hybrid,
}

/// Physical address value object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// Street address (line 1)
    pub street1: String,

    /// Street address (line 2) - optional
    pub street2: Option<String>,

    /// City/locality
    pub locality: String,

    /// State/province/region
    pub region: String,

    /// Country
    pub country: String,

    /// Postal/ZIP code
    pub postal_code: String,
}

/// Geographic coordinates value object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoCoordinates {
    /// Latitude in decimal degrees (-90 to 90)
    pub latitude: f64,

    /// Longitude in decimal degrees (-180 to 180)
    pub longitude: f64,

    /// Altitude in meters (optional)
    pub altitude: Option<f64>,

    /// Coordinate system (default: WGS84)
    pub coordinate_system: String,
}

/// Virtual location details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualLocation {
    /// Platform or system hosting the virtual location
    pub platform: String,

    /// Identifier within the platform
    pub platform_id: String,

    /// Access URL if applicable
    pub url: Option<String>,

    /// Additional platform-specific data
    pub platform_data: HashMap<String, String>,
}

impl Location {
    /// Create a new physical location with an address
    pub fn new_physical(
        id: EntityId<LocationMarker>,
        name: String,
        address: Address,
    ) -> DomainResult<Self> {
        address.validate()?;

        Ok(Self {
            entity: Entity::with_id(id),
            version: 0,
            name,
            location_type: LocationType::Physical,
            address: Some(address),
            coordinates: None,
            virtual_location: None,
            parent_id: None,
            metadata: HashMap::new(),
        })
    }

    /// Create a new virtual location
    pub fn new_virtual(
        id: EntityId<LocationMarker>,
        name: String,
        virtual_location: VirtualLocation,
    ) -> DomainResult<Self> {
        Ok(Self {
            entity: Entity::with_id(id),
            version: 0,
            name,
            location_type: LocationType::Virtual,
            address: None,
            coordinates: None,
            virtual_location: Some(virtual_location),
            parent_id: None,
            metadata: HashMap::new(),
        })
    }

    /// Create a new location with just coordinates
    pub fn new_from_coordinates(
        id: EntityId<LocationMarker>,
        name: String,
        coordinates: GeoCoordinates,
    ) -> DomainResult<Self> {
        coordinates.validate()?;

        Ok(Self {
            entity: Entity::with_id(id),
            version: 0,
            name,
            location_type: LocationType::Physical,
            address: None,
            coordinates: Some(coordinates),
            virtual_location: None,
            parent_id: None,
            metadata: HashMap::new(),
        })
    }

    /// Set the address for this location
    pub fn set_address(&mut self, address: Address) -> DomainResult<()> {
        address.validate()?;

        if self.location_type == LocationType::Virtual {
            return Err(DomainError::ValidationError(
                "Cannot set physical address on virtual location".to_string()
            ));
        }

        self.address = Some(address);
        self.entity.touch();
        Ok(())
    }

    /// Set geographic coordinates
    pub fn set_coordinates(&mut self, coordinates: GeoCoordinates) -> DomainResult<()> {
        coordinates.validate()?;

        if self.location_type == LocationType::Virtual {
            return Err(DomainError::ValidationError(
                "Cannot set coordinates on virtual location".to_string()
            ));
        }

        self.coordinates = Some(coordinates);
        self.entity.touch();
        Ok(())
    }

    /// Set parent location for hierarchical structures
    pub fn set_parent(&mut self, parent_id: EntityId<LocationMarker>) -> DomainResult<()> {
        // Prevent self-reference
        if parent_id == self.entity.id {
            return Err(DomainError::ValidationError(
                "Location cannot be its own parent".to_string()
            ));
        }

        self.parent_id = Some(parent_id);
        self.entity.touch();
        Ok(())
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.entity.touch();
    }
}

impl AggregateRoot for Location {
    type Id = EntityId<LocationMarker>;

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

impl Address {
    /// Create a new address
    pub fn new(
        street1: String,
        locality: String,
        region: String,
        country: String,
        postal_code: String,
    ) -> Self {
        Self {
            street1,
            street2: None,
            locality,
            region,
            country,
            postal_code,
        }
    }

    /// Add second street line
    pub fn with_street2(mut self, street2: String) -> Self {
        self.street2 = Some(street2);
        self
    }

    /// Validate address invariants
    pub fn validate(&self) -> DomainResult<()> {
        if self.street1.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Street address cannot be empty".to_string()
            ));
        }

        if self.locality.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Locality cannot be empty".to_string()
            ));
        }

        if self.region.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Region cannot be empty".to_string()
            ));
        }

        if self.country.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Country cannot be empty".to_string()
            ));
        }

        if self.postal_code.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Postal code cannot be empty".to_string()
            ));
        }

        // Additional validation could include:
        // - Country-specific postal code formats
        // - Valid country codes
        // - Region validation based on country

        Ok(())
    }

    /// Format as single-line string
    pub fn format_single_line(&self) -> String {
        let mut parts = vec![self.street1.clone()];

        if let Some(street2) = &self.street2 {
            parts.push(street2.clone());
        }

        parts.push(format!("{}, {} {}", self.locality, self.region, self.postal_code));
        parts.push(self.country.clone());

        parts.join(", ")
    }
}

impl GeoCoordinates {
    /// Create new coordinates (defaults to WGS84)
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude: None,
            coordinate_system: "WGS84".to_string(),
        }
    }

    /// Add altitude
    pub fn with_altitude(mut self, altitude: f64) -> Self {
        self.altitude = Some(altitude);
        self
    }

    /// Use different coordinate system
    pub fn with_coordinate_system(mut self, system: String) -> Self {
        self.coordinate_system = system;
        self
    }

    /// Validate coordinate ranges
    pub fn validate(&self) -> DomainResult<()> {
        if self.latitude < -90.0 || self.latitude > 90.0 {
            return Err(DomainError::ValidationError(
                format!("Latitude {} is out of range [-90, 90]", self.latitude)
            ));
        }

        if self.longitude < -180.0 || self.longitude > 180.0 {
            return Err(DomainError::ValidationError(
                format!("Longitude {} is out of range [-180, 180]", self.longitude)
            ));
        }

        Ok(())
    }

    /// Calculate distance to another point (in meters, using Haversine formula)
    pub fn distance_to(&self, other: &GeoCoordinates) -> f64 {
        const EARTH_RADIUS_M: f64 = 6_371_000.0;

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS_M * c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_validation() {
        // Valid address
        let valid_address = Address::new(
            "123 Main St".to_string(),
            "Springfield".to_string(),
            "IL".to_string(),
            "USA".to_string(),
            "62701".to_string(),
        );
        assert!(valid_address.validate().is_ok());

        // Invalid address - empty street
        let invalid_address = Address::new(
            "".to_string(),
            "Springfield".to_string(),
            "IL".to_string(),
            "USA".to_string(),
            "62701".to_string(),
        );
        assert!(invalid_address.validate().is_err());
    }

    #[test]
    fn test_geo_coordinates_validation() {
        // Valid coordinates
        let valid_coords = GeoCoordinates::new(40.7128, -74.0060);
        assert!(valid_coords.validate().is_ok());

        // Invalid latitude
        let invalid_lat = GeoCoordinates::new(91.0, -74.0060);
        assert!(invalid_lat.validate().is_err());

        // Invalid longitude
        let invalid_lon = GeoCoordinates::new(40.7128, -181.0);
        assert!(invalid_lon.validate().is_err());
    }

    #[test]
    fn test_location_creation() {
        let location_id = EntityId::<LocationMarker>::new();

        let address = Address::new(
            "1 Infinite Loop".to_string(),
            "Cupertino".to_string(),
            "CA".to_string(),
            "USA".to_string(),
            "95014".to_string(),
        );

        let location = Location::new_physical(
            location_id,
            "Apple Park".to_string(),
            address,
        ).unwrap();

        assert_eq!(location.name, "Apple Park");
        assert_eq!(location.location_type, LocationType::Physical);
        assert!(location.address.is_some());
    }

    #[test]
    fn test_distance_calculation() {
        // New York City
        let nyc = GeoCoordinates::new(40.7128, -74.0060);

        // Los Angeles
        let la = GeoCoordinates::new(34.0522, -118.2437);

        let distance = nyc.distance_to(&la);

        // Should be approximately 3,944 km
        assert!((distance - 3_944_000.0).abs() < 10_000.0);
    }
}
