//! Organization aggregate for managing organizational structures, roles, and locations
//!
//! This module provides a comprehensive organization model supporting:
//! - Hierarchical organizational units (departments, teams, divisions)
//! - Role-based member assignments
//! - Multiple location associations
//! - Component-based extensibility

use crate::{
    entity::{AggregateRoot, Entity, EntityId},
    errors::{DomainError, DomainResult},
    component::{Component, ComponentStorage},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Marker type for Organization entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrganizationMarker;

/// Organization aggregate root
#[derive(Debug, Clone)]
pub struct Organization {
    /// Entity base with ID and version
    entity: Entity<OrganizationMarker>,

    /// Version for optimistic concurrency control
    version: u64,

    /// Organization name
    pub name: String,

    /// Organization type
    pub org_type: OrganizationType,

    /// Parent organization (for hierarchical structures)
    pub parent_id: Option<EntityId<OrganizationMarker>>,

    /// Child organizational units
    pub child_units: HashSet<EntityId<OrganizationMarker>>,

    /// Members with their roles
    members: HashMap<Uuid, OrganizationMember>,

    /// Locations associated with this organization
    pub locations: HashSet<Uuid>,

    /// Primary/headquarters location
    pub primary_location: Option<Uuid>,

    /// Dynamic components for extensibility
    pub components: ComponentStorage,

    /// Organization status
    pub status: OrganizationStatus,
}

/// Types of organizations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganizationType {
    /// Top-level company or corporation
    Company,
    /// Major division within a company
    Division,
    /// Department within a division
    Department,
    /// Team within a department
    Team,
    /// Project-based organization
    Project,
    /// External partner organization
    Partner,
    /// Customer organization
    Customer,
    /// Vendor/supplier organization
    Vendor,
    /// Custom organization type
    Custom,
}

/// Organization status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganizationStatus {
    /// Actively operating
    Active,
    /// Temporarily inactive
    Inactive,
    /// In the process of being set up
    Pending,
    /// Archived/historical
    Archived,
}

/// Member of an organization with role assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    /// Person ID
    pub person_id: Uuid,

    /// Role within the organization
    pub role: OrganizationRole,

    /// When they joined
    pub joined_at: chrono::DateTime<chrono::Utc>,

    /// When their role ends (if temporary)
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Reports to (manager/supervisor)
    pub reports_to: Option<Uuid>,

    /// Additional role metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Roles within an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationRole {
    /// Role identifier
    pub role_id: String,

    /// Display name
    pub title: String,

    /// Role level/seniority
    pub level: RoleLevel,

    /// Permissions associated with this role
    pub permissions: HashSet<String>,

    /// Role-specific attributes
    pub attributes: HashMap<String, String>,
}

/// Role levels for hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RoleLevel {
    /// C-level executives (CEO, CTO, CFO, etc.)
    Executive,
    /// Directors overseeing multiple teams or departments
    Director,
    /// Managers responsible for teams
    Manager,
    /// Team leads with technical leadership responsibilities
    Lead,
    /// Senior individual contributors
    Senior,
    /// Mid-level individual contributors
    Mid,
    /// Junior individual contributors
    Junior,
    /// Interns and trainees
    Intern,
}

/// Size classification for organizations based on employee count
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SizeCategory {
    /// 1-10 employees
    Startup,
    /// 11-50 employees
    Small,
    /// 51-250 employees
    Medium,
    /// 251-1000 employees
    Large,
    /// 1000+ employees
    Enterprise,
}

impl Organization {
    /// Create a new organization
    pub fn new(name: String, org_type: OrganizationType) -> Self {
        Self {
            entity: Entity::new(),
            version: 0,
            name,
            org_type,
            parent_id: None,
            child_units: HashSet::new(),
            members: HashMap::new(),
            locations: HashSet::new(),
            primary_location: None,
            components: ComponentStorage::new(),
            status: OrganizationStatus::Active,
        }
    }

    /// Create with specific ID
    pub fn with_id(id: EntityId<OrganizationMarker>, name: String, org_type: OrganizationType) -> Self {
        Self {
            entity: Entity::with_id(id),
            version: 0,
            name,
            org_type,
            parent_id: None,
            child_units: HashSet::new(),
            members: HashMap::new(),
            locations: HashSet::new(),
            primary_location: None,
            components: ComponentStorage::new(),
            status: OrganizationStatus::Active,
        }
    }

    /// Set parent organization (for hierarchical structure)
    pub fn set_parent(&mut self, parent_id: EntityId<OrganizationMarker>) -> DomainResult<()> {
        // Prevent self-reference
        if parent_id == self.entity.id {
            return Err(DomainError::ValidationError("Organization cannot be its own parent".to_string()));
        }

        // Prevent circular references (would need full graph check in practice)
        if self.child_units.contains(&parent_id) {
            return Err(DomainError::ValidationError("Circular reference detected".to_string()));
        }

        self.parent_id = Some(parent_id);
        self.entity.touch();
        Ok(())
    }

    /// Add a child organizational unit
    pub fn add_child_unit(&mut self, child_id: EntityId<OrganizationMarker>) -> DomainResult<()> {
        // Prevent self-reference
        if child_id == self.entity.id {
            return Err(DomainError::ValidationError("Organization cannot be its own child".to_string()));
        }

        // Prevent circular references
        if Some(child_id) == self.parent_id {
            return Err(DomainError::ValidationError("Parent cannot be a child".to_string()));
        }

        self.child_units.insert(child_id);
        self.entity.touch();
        Ok(())
    }

    /// Remove a child organizational unit
    pub fn remove_child_unit(&mut self, child_id: &EntityId<OrganizationMarker>) -> bool {
        let removed = self.child_units.remove(child_id);
        if removed {
            self.entity.touch();
        }
        removed
    }

    /// Add a member to the organization
    pub fn add_member(&mut self, person_id: Uuid, role: OrganizationRole) -> DomainResult<()> {
        // Check if already a member
        if self.members.contains_key(&person_id) {
            return Err(DomainError::ComponentAlreadyExists(format!("Person {} is already a member", person_id)));
        }

        let member = OrganizationMember {
            person_id,
            role,
            joined_at: chrono::Utc::now(),
            ends_at: None,
            reports_to: None,
            metadata: HashMap::new(),
        };

        self.members.insert(person_id, member);
        self.entity.touch();
        Ok(())
    }

    /// Update a member's role
    pub fn update_member_role(&mut self, person_id: Uuid, new_role: OrganizationRole) -> DomainResult<()> {
        match self.members.get_mut(&person_id) {
            Some(member) => {
                member.role = new_role;
                self.entity.touch();
                Ok(())
            }
            None => Err(DomainError::EntityNotFound {
                entity_type: "OrganizationMember".to_string(),
                id: person_id.to_string(),
            }),
        }
    }

    /// Set reporting relationship
    pub fn set_reports_to(&mut self, person_id: Uuid, manager_id: Uuid) -> DomainResult<()> {
        // Validate manager exists
        if !self.members.contains_key(&manager_id) {
            return Err(DomainError::EntityNotFound {
                entity_type: "Manager".to_string(),
                id: manager_id.to_string(),
            });
        }

        // Prevent self-reporting
        if person_id == manager_id {
            return Err(DomainError::ValidationError("Person cannot report to themselves".to_string()));
        }

        // Prevent circular reporting (simplified - would need full graph check)
        if let Some(manager) = self.members.get(&manager_id) {
            if manager.reports_to == Some(person_id) {
                return Err(DomainError::ValidationError("Circular reporting relationship detected".to_string()));
            }
        }

        match self.members.get_mut(&person_id) {
            Some(member) => {
                member.reports_to = Some(manager_id);
                self.entity.touch();
                Ok(())
            }
            None => Err(DomainError::EntityNotFound {
                entity_type: "OrganizationMember".to_string(),
                id: person_id.to_string(),
            }),
        }
    }

    /// Remove a member from the organization
    pub fn remove_member(&mut self, person_id: &Uuid) -> Option<OrganizationMember> {
        let removed = self.members.remove(person_id);
        if removed.is_some() {
            // Remove any reporting relationships
            for member in self.members.values_mut() {
                if member.reports_to == Some(*person_id) {
                    member.reports_to = None;
                }
            }
            self.entity.touch();
        }
        removed
    }

    /// Add a location to the organization
    pub fn add_location(&mut self, location_id: Uuid) -> bool {
        let added = self.locations.insert(location_id);
        if added {
            // Set as primary if it's the first location
            if self.locations.len() == 1 && self.primary_location.is_none() {
                self.primary_location = Some(location_id);
            }
            self.entity.touch();
        }
        added
    }

    /// Set primary location
    pub fn set_primary_location(&mut self, location_id: Uuid) -> DomainResult<()> {
        if !self.locations.contains(&location_id) {
            return Err(DomainError::EntityNotFound {
                entity_type: "Location".to_string(),
                id: location_id.to_string(),
            });
        }

        self.primary_location = Some(location_id);
        self.entity.touch();
        Ok(())
    }

    /// Remove a location
    pub fn remove_location(&mut self, location_id: &Uuid) -> bool {
        let removed = self.locations.remove(location_id);
        if removed {
            // Clear primary if it was removed
            if self.primary_location == Some(*location_id) {
                self.primary_location = None;
            }
            self.entity.touch();
        }
        removed
    }

    /// Get all members
    pub fn members(&self) -> &HashMap<Uuid, OrganizationMember> {
        &self.members
    }

    /// Get members by role
    pub fn members_by_role(&self, role_id: &str) -> Vec<&OrganizationMember> {
        self.members
            .values()
            .filter(|m| m.role.role_id == role_id)
            .collect()
    }

    /// Get direct reports for a person
    pub fn direct_reports(&self, person_id: Uuid) -> Vec<&OrganizationMember> {
        self.members
            .values()
            .filter(|m| m.reports_to == Some(person_id))
            .collect()
    }

    /// Get organization hierarchy depth
    pub fn hierarchy_depth(&self) -> usize {
        // Count unique reporting levels
        let mut levels = HashSet::new();
        let mut max_depth = 0;

        for member in self.members.values() {
            let depth = self.calculate_member_depth(member.person_id, 0);
            max_depth = max_depth.max(depth);
            levels.insert(member.role.level);
        }

        max_depth
    }

    fn calculate_member_depth(&self, person_id: Uuid, current_depth: usize) -> usize {
        let reports = self.direct_reports(person_id);
        if reports.is_empty() {
            current_depth
        } else {
            reports
                .iter()
                .map(|r| self.calculate_member_depth(r.person_id, current_depth + 1))
                .max()
                .unwrap_or(current_depth)
        }
    }

    /// Check if organization is empty (no members)
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get location count
    pub fn location_count(&self) -> usize {
        self.locations.len()
    }
}

impl AggregateRoot for Organization {
    type Id = EntityId<OrganizationMarker>;

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

/// Component for organization metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMetadata {
    /// Industry sector (e.g., "Technology", "Healthcare", "Finance")
    pub industry: Option<String>,
    /// Size classification based on employee count
    pub size_category: Option<SizeCategory>,
    /// Date when the organization was founded
    pub founded_date: Option<chrono::NaiveDate>,
    /// Organization's primary website URL
    pub website: Option<String>,
    /// Tax identification number
    pub tax_id: Option<String>,
    /// Business registration number
    pub registration_number: Option<String>,
}

impl Component for OrganizationMetadata {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "OrganizationMetadata"
    }
}

/// Component for budget information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetComponent {
    /// The fiscal year this budget applies to
    pub fiscal_year: i32,
    /// Total budget amount
    pub total_budget: f64,
    /// Currency code (e.g., "USD", "EUR", "GBP")
    pub currency: String,
    /// Amount already allocated to projects/departments
    pub allocated: f64,
    /// Amount already spent
    pub spent: f64,
}

impl Component for BudgetComponent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "BudgetComponent"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organization_creation() {
        let org = Organization::new("Acme Corp".to_string(), OrganizationType::Company);
        assert_eq!(org.name, "Acme Corp");
        assert_eq!(org.org_type, OrganizationType::Company);
        assert_eq!(org.status, OrganizationStatus::Active);
        assert!(org.members.is_empty());
        assert!(org.locations.is_empty());
    }

    #[test]
    fn test_organizational_hierarchy() {
        let company_id = EntityId::<OrganizationMarker>::new();
        let dept_id = EntityId::<OrganizationMarker>::new();

        let mut company = Organization::with_id(company_id, "TechCorp".to_string(), OrganizationType::Company);
        let mut department = Organization::with_id(dept_id, "Engineering".to_string(), OrganizationType::Department);

        // Set up hierarchy
        department.set_parent(company_id).unwrap();
        company.add_child_unit(dept_id).unwrap();

        assert_eq!(department.parent_id, Some(company_id));
        assert!(company.child_units.contains(&dept_id));

        // Test circular reference prevention
        assert!(company.set_parent(dept_id).is_err());
        assert!(department.add_child_unit(company_id).is_err());
    }

    #[test]
    fn test_member_management() {
        let mut org = Organization::new("Engineering".to_string(), OrganizationType::Department);

        let person1 = Uuid::new_v4();
        let person2 = Uuid::new_v4();

        let manager_role = OrganizationRole {
            role_id: "eng_manager".to_string(),
            title: "Engineering Manager".to_string(),
            level: RoleLevel::Manager,
            permissions: ["approve_pr", "manage_team"].iter().map(|s| s.to_string()).collect(),
            attributes: HashMap::new(),
        };

        let engineer_role = OrganizationRole {
            role_id: "engineer".to_string(),
            title: "Software Engineer".to_string(),
            level: RoleLevel::Mid,
            permissions: ["create_pr", "deploy"].iter().map(|s| s.to_string()).collect(),
            attributes: HashMap::new(),
        };

        // Add members
        org.add_member(person1, manager_role).unwrap();
        org.add_member(person2, engineer_role).unwrap();

        assert_eq!(org.member_count(), 2);

        // Set reporting relationship
        org.set_reports_to(person2, person1).unwrap();

        // Check direct reports
        let reports = org.direct_reports(person1);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].person_id, person2);

        // Test self-reporting prevention
        assert!(org.set_reports_to(person1, person1).is_err());

        // Test circular reporting prevention
        assert!(org.set_reports_to(person1, person2).is_err());
    }

    #[test]
    fn test_location_management() {
        let mut org = Organization::new("Acme Corp".to_string(), OrganizationType::Company);

        let loc1 = Uuid::new_v4();
        let loc2 = Uuid::new_v4();
        let loc3 = Uuid::new_v4();

        // Add locations
        assert!(org.add_location(loc1));
        assert!(org.add_location(loc2));
        assert!(!org.add_location(loc1)); // Duplicate

        // First location becomes primary automatically
        assert_eq!(org.primary_location, Some(loc1));

        // Change primary
        org.set_primary_location(loc2).unwrap();
        assert_eq!(org.primary_location, Some(loc2));

        // Can't set non-existent location as primary
        assert!(org.set_primary_location(loc3).is_err());

        // Remove location
        assert!(org.remove_location(&loc1));
        assert_eq!(org.location_count(), 1);

        // Remove primary location
        assert!(org.remove_location(&loc2));
        assert_eq!(org.primary_location, None);
    }

    #[test]
    fn test_organization_components() {
        let mut org = Organization::new("StartupCo".to_string(), OrganizationType::Company);

        let metadata = OrganizationMetadata {
            industry: Some("Technology".to_string()),
            size_category: Some(SizeCategory::Startup),
            founded_date: Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            website: Some("https://startupco.com".to_string()),
            tax_id: Some("12-3456789".to_string()),
            registration_number: None,
        };

        org.components.add(metadata).unwrap();

        let budget = BudgetComponent {
            fiscal_year: 2024,
            total_budget: 1_000_000.0,
            currency: "USD".to_string(),
            allocated: 800_000.0,
            spent: 250_000.0,
        };

        org.components.add(budget).unwrap();

        assert!(org.components.has::<OrganizationMetadata>());
        assert!(org.components.has::<BudgetComponent>());
    }
}
