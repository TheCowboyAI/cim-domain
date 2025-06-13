//! Person aggregate and related components
//!
//! A Person is an aggregate with an ID and various components that can be
//! composed to create different views (Employee, Customer, etc.)

use crate::{AggregateRoot, Entity, EntityId, DomainError, DomainResult, Component, ComponentStorage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::any::Any;

/// Person aggregate - represents an individual with composable components
#[derive(Debug, Clone)]
pub struct Person {
    /// Core entity data
    entity: Entity<PersonMarker>,

    /// Version for optimistic concurrency control
    version: u64,

    /// Components attached to this person
    components: ComponentStorage,

    /// Component metadata (when added, by whom, etc.)
    component_metadata: HashMap<String, ComponentMetadata>,
}

/// Marker type for Person entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PersonMarker;

/// Metadata about when and why a component was added
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetadata {
    /// When this component was added
    pub added_at: std::time::SystemTime,

    /// Who added this component
    pub added_by: String,

    /// Reason or context for adding
    pub reason: Option<String>,
}

// Common person components

/// Basic identity information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityComponent {
    /// Legal name
    pub legal_name: String,

    /// Preferred name (if different from legal)
    pub preferred_name: Option<String>,

    /// Date of birth
    pub date_of_birth: Option<chrono::NaiveDate>,

    /// Government ID number (SSN, etc.)
    pub government_id: Option<String>,
}

/// Contact information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactComponent {
    /// Email addresses
    pub emails: Vec<EmailAddress>,

    /// Phone numbers
    pub phones: Vec<PhoneNumber>,

    /// Physical addresses
    pub addresses: Vec<Uuid>, // References to Location aggregates
}

/// Email address with type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailAddress {
    /// Email address
    pub email: String,

    /// Type (work, personal, etc.)
    pub email_type: String,

    /// Is this the primary email?
    pub is_primary: bool,

    /// Is this verified?
    pub is_verified: bool,
}

/// Phone number with type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneNumber {
    /// Phone number (E.164 format preferred)
    pub number: String,

    /// Type (mobile, work, home, etc.)
    pub phone_type: String,

    /// Is this the primary phone?
    pub is_primary: bool,

    /// Can receive SMS?
    pub sms_capable: bool,
}

/// Employment information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmploymentComponent {
    /// Organization ID
    pub organization_id: Uuid,

    /// Employee ID within the organization
    pub employee_id: String,

    /// Job title
    pub title: String,

    /// Department
    pub department: Option<String>,

    /// Manager's person ID
    pub manager_id: Option<Uuid>,

    /// Employment status (active, terminated, on_leave, etc.)
    pub status: String,

    /// Start date
    pub start_date: chrono::NaiveDate,

    /// End date (if terminated)
    pub end_date: Option<chrono::NaiveDate>,
}

/// Position/role information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionComponent {
    /// Position ID
    pub position_id: Uuid,

    /// Position title
    pub title: String,

    /// Level/grade
    pub level: Option<String>,

    /// Responsibilities
    pub responsibilities: Vec<String>,

    /// Required skills
    pub required_skills: Vec<String>,

    /// Start date in this position
    pub start_date: chrono::NaiveDate,

    /// End date (if no longer in position)
    pub end_date: Option<chrono::NaiveDate>,
}

/// Skills and qualifications
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillsComponent {
    /// Skills with proficiency levels
    pub skills: HashMap<String, SkillProficiency>,

    /// Certifications
    pub certifications: Vec<Certification>,

    /// Education
    pub education: Vec<Education>,
}

/// Skill proficiency level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillProficiency {
    /// Skill name
    pub skill: String,

    /// Proficiency level (1-5, beginner/intermediate/expert, etc.)
    pub level: String,

    /// Years of experience
    pub years_experience: Option<f32>,

    /// Last used date
    pub last_used: Option<chrono::NaiveDate>,
}

/// Certification information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certification {
    /// Certification name
    pub name: String,

    /// Issuing organization
    pub issuer: String,

    /// Issue date
    pub issue_date: chrono::NaiveDate,

    /// Expiry date (if applicable)
    pub expiry_date: Option<chrono::NaiveDate>,

    /// Credential ID
    pub credential_id: Option<String>,
}

/// Education information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Education {
    /// Institution name
    pub institution: String,

    /// Degree/qualification
    pub degree: String,

    /// Field of study
    pub field_of_study: Option<String>,

    /// Start date
    pub start_date: chrono::NaiveDate,

    /// End date
    pub end_date: Option<chrono::NaiveDate>,

    /// Grade/GPA
    pub grade: Option<String>,
}

/// Access control and permissions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessComponent {
    /// Roles assigned to this person
    pub roles: Vec<String>,

    /// Direct permissions
    pub permissions: Vec<String>,

    /// Groups this person belongs to
    pub groups: Vec<Uuid>,

    /// Access level/clearance
    pub access_level: Option<String>,
}

/// External system identifiers (for projections)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalIdentifiersComponent {
    /// LDAP distinguished name
    pub ldap_dn: Option<String>,

    /// Active Directory SID
    pub ad_sid: Option<String>,

    /// OAuth subject identifiers
    pub oauth_subjects: HashMap<String, String>,

    /// Other system IDs
    pub external_ids: HashMap<String, String>,
}

impl Person {
    /// Create a new person with basic identity
    pub fn new(id: EntityId<PersonMarker>, identity: IdentityComponent) -> Self {
        let mut components = ComponentStorage::new();
        components.add(identity).unwrap();

        let mut component_metadata = HashMap::new();
        component_metadata.insert(
            "Identity".to_string(),
            ComponentMetadata {
                added_at: std::time::SystemTime::now(),
                added_by: "system".to_string(),
                reason: Some("Initial identity".to_string()),
            },
        );

        Self {
            entity: Entity::with_id(id),
            version: 0,
            components,
            component_metadata,
        }
    }

    /// Add a component to this person
    pub fn add_component<C: Component + 'static>(
        &mut self,
        component: C,
        added_by: &str,
        reason: Option<String>,
    ) -> DomainResult<()> {
        let component_type = component.type_name().to_string();

        // Add the component
        self.components.add(component)?;

        // Add metadata
        self.component_metadata.insert(
            component_type,
            ComponentMetadata {
                added_at: std::time::SystemTime::now(),
                added_by: added_by.to_string(),
                reason,
            },
        );

        self.entity.touch();
        self.version += 1;

        Ok(())
    }

    /// Remove a component
    pub fn remove_component<C: Component + 'static>(&mut self) -> DomainResult<()> {
        let component_type = std::any::type_name::<C>();

        if self.components.remove::<C>().is_some() {
            self.component_metadata.remove(component_type);
            self.entity.touch();
            self.version += 1;
            Ok(())
        } else {
            Err(DomainError::ComponentNotFound(format!(
                "Component {} not found",
                component_type
            )))
        }
    }

    /// Get a component
    pub fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components.get::<C>()
    }

    /// Check if person has a component
    pub fn has_component<C: Component + 'static>(&self) -> bool {
        self.components.has::<C>()
    }

    /// Get all component types
    pub fn component_types(&self) -> Vec<String> {
        self.component_metadata.keys().cloned().collect()
    }
}

impl AggregateRoot for Person {
    type Id = EntityId<PersonMarker>;

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

// Component trait implementations

impl Component for IdentityComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Identity"
    }
}

impl Component for ContactComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Contact"
    }
}

impl Component for EmploymentComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Employment"
    }
}

impl Component for PositionComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Position"
    }
}

impl Component for SkillsComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Skills"
    }
}

impl Component for AccessComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Access"
    }
}

impl Component for ExternalIdentifiersComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ExternalIdentifiers"
    }
}

// View projections

/// Employee view of a person
pub struct EmployeeView {
    /// The person's unique identifier
    pub person_id: EntityId<PersonMarker>,
    /// Identity information (name, DOB, etc.)
    pub identity: IdentityComponent,
    /// Contact information (email, phone, address)
    pub contact: ContactComponent,
    /// Employment details (organization, title, department)
    pub employment: EmploymentComponent,
    /// Current position information if available
    pub position: Option<PositionComponent>,
    /// Skills and certifications if available
    pub skills: Option<SkillsComponent>,
}

impl EmployeeView {
    /// Create employee view from person
    pub fn from_person(person: &Person) -> DomainResult<Self> {
        let identity = person.get_component::<IdentityComponent>()
            .ok_or_else(|| DomainError::ValidationError(
                "Person missing identity component".to_string()
            ))?
            .clone();

        let contact = person.get_component::<ContactComponent>()
            .ok_or_else(|| DomainError::ValidationError(
                "Employee missing contact component".to_string()
            ))?
            .clone();

        let employment = person.get_component::<EmploymentComponent>()
            .ok_or_else(|| DomainError::ValidationError(
                "Employee missing employment component".to_string()
            ))?
            .clone();

        Ok(Self {
            person_id: person.id(),
            identity,
            contact,
            employment,
            position: person.get_component::<PositionComponent>().cloned(),
            skills: person.get_component::<SkillsComponent>().cloned(),
        })
    }
}

/// LDAP projection
pub struct LdapProjection {
    /// Distinguished Name (full LDAP path)
    pub dn: String,
    /// Common Name (typically the preferred name)
    pub cn: String,
    /// Surname (last name)
    pub sn: String,
    /// Given name (first name)
    pub given_name: String,
    /// Email addresses
    pub mail: Vec<String>,
    /// Phone numbers
    pub telephone_number: Vec<String>,
    /// Job title if employed
    pub title: Option<String>,
    /// Department if employed
    pub department: Option<String>,
    /// Manager's DN if applicable
    pub manager: Option<String>,
}

impl LdapProjection {
    /// Create LDAP projection from person
    pub fn from_person(person: &Person, base_dn: &str) -> DomainResult<Self> {
        let identity = person.get_component::<IdentityComponent>()
            .ok_or_else(|| DomainError::ValidationError(
                "Cannot project to LDAP without identity".to_string()
            ))?;

        let contact = person.get_component::<ContactComponent>()
            .ok_or_else(|| DomainError::ValidationError(
                "Cannot project to LDAP without contact".to_string()
            ))?;

        let employment = person.get_component::<EmploymentComponent>();

        // Parse name
        let name_parts: Vec<&str> = identity.legal_name.split_whitespace().collect();
        let given_name = name_parts.first().unwrap_or(&"").to_string();
        let sn = name_parts.last().unwrap_or(&"").to_string();

        // Build DN
        let cn = identity.preferred_name.as_ref()
            .unwrap_or(&identity.legal_name);
        let dn = format!("cn={},{}", cn, base_dn);

        // Collect emails and phones
        let mail: Vec<String> = contact.emails.iter()
            .map(|e| e.email.clone())
            .collect();

        let telephone_number: Vec<String> = contact.phones.iter()
            .map(|p| p.number.clone())
            .collect();

        Ok(Self {
            dn,
            cn: cn.clone(),
            sn,
            given_name,
            mail,
            telephone_number,
            title: employment.map(|e| e.title.clone()),
            department: employment.and_then(|e| e.department.clone()),
            manager: employment.and_then(|e| e.manager_id.map(|id| id.to_string())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_creation() {
        let person_id = EntityId::<PersonMarker>::new();
        let identity = IdentityComponent {
            legal_name: "John Doe".to_string(),
            preferred_name: Some("Johnny".to_string()),
            date_of_birth: None,
            government_id: None,
        };

        let person = Person::new(person_id, identity.clone());

        assert_eq!(person.id(), person_id);
        assert_eq!(person.version(), 0);
        assert!(person.has_component::<IdentityComponent>());

        let stored_identity = person.get_component::<IdentityComponent>().unwrap();
        assert_eq!(stored_identity.legal_name, "John Doe");
    }

    #[test]
    fn test_component_management() {
        let person_id = EntityId::<PersonMarker>::new();
        let identity = IdentityComponent {
            legal_name: "Jane Smith".to_string(),
            preferred_name: None,
            date_of_birth: None,
            government_id: None,
        };

        let mut person = Person::new(person_id, identity);

        // Add contact component
        let contact = ContactComponent {
            emails: vec![EmailAddress {
                email: "jane@example.com".to_string(),
                email_type: "work".to_string(),
                is_primary: true,
                is_verified: true,
            }],
            phones: vec![],
            addresses: vec![],
        };

        person.add_component(contact.clone(), "admin", Some("Initial setup".to_string())).unwrap();

        assert!(person.has_component::<ContactComponent>());
        assert_eq!(person.version(), 1);

        // Remove component
        person.remove_component::<ContactComponent>().unwrap();
        assert!(!person.has_component::<ContactComponent>());
        assert_eq!(person.version(), 2);
    }

    #[test]
    fn test_employee_view() {
        let person_id = EntityId::<PersonMarker>::new();
        let identity = IdentityComponent {
            legal_name: "Alice Johnson".to_string(),
            preferred_name: None,
            date_of_birth: None,
            government_id: None,
        };

        let mut person = Person::new(person_id, identity);

        // Add required components for employee view
        let contact = ContactComponent {
            emails: vec![],
            phones: vec![],
            addresses: vec![],
        };
        person.add_component(contact, "system", None).unwrap();

        let employment = EmploymentComponent {
            organization_id: Uuid::new_v4(),
            employee_id: "EMP001".to_string(),
            title: "Software Engineer".to_string(),
            department: Some("Engineering".to_string()),
            manager_id: None,
            status: "active".to_string(),
            start_date: chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            end_date: None,
        };
        person.add_component(employment, "hr", None).unwrap();

        // Create employee view
        let employee_view = EmployeeView::from_person(&person).unwrap();
        assert_eq!(employee_view.person_id, person_id);
        assert_eq!(employee_view.employment.title, "Software Engineer");
    }

    #[test]
    fn test_ldap_projection() {
        let person_id = EntityId::<PersonMarker>::new();
        let identity = IdentityComponent {
            legal_name: "Bob Wilson".to_string(),
            preferred_name: Some("Bobby".to_string()),
            date_of_birth: None,
            government_id: None,
        };

        let mut person = Person::new(person_id, identity);

        let contact = ContactComponent {
            emails: vec![EmailAddress {
                email: "bob@company.com".to_string(),
                email_type: "work".to_string(),
                is_primary: true,
                is_verified: true,
            }],
            phones: vec![PhoneNumber {
                number: "+1234567890".to_string(),
                phone_type: "work".to_string(),
                is_primary: true,
                sms_capable: false,
            }],
            addresses: vec![],
        };
        person.add_component(contact, "system", None).unwrap();

        let ldap = LdapProjection::from_person(&person, "ou=users,dc=company,dc=com").unwrap();

        assert_eq!(ldap.cn, "Bobby");
        assert_eq!(ldap.sn, "Wilson");
        assert_eq!(ldap.given_name, "Bob");
        assert_eq!(ldap.dn, "cn=Bobby,ou=users,dc=company,dc=com");
        assert_eq!(ldap.mail.len(), 1);
        assert_eq!(ldap.mail[0], "bob@company.com");
    }
}
