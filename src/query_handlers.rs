//! Query handlers for CIM domain aggregates
//!
//! Query handlers process queries and return data from read models/projections.
//! They implement the read side of CQRS, providing optimized data access.

use crate::{
    cqrs::{Query, QueryHandler as CqrsQueryHandler, QueryEnvelope, QueryAcknowledgment, QueryStatus, QueryId},
    errors::DomainResult,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Query result type
pub type QueryResult<T> = Result<T, String>;

/// Event publisher trait for publishing query results
pub trait EventPublisher: Send + Sync {
    /// Publish query results as events
    fn publish_query_result(&self, query_id: QueryId, result: serde_json::Value) -> DomainResult<()>;
}

/// Mock event publisher for testing
#[derive(Clone)]
pub struct MockEventPublisher;

impl EventPublisher for MockEventPublisher {
    fn publish_query_result(&self, _query_id: QueryId, _result: serde_json::Value) -> DomainResult<()> {
        Ok(())
    }
}

/// Query handler trait that returns data directly (for internal use)
pub trait DirectQueryHandler<Q, R> {
    /// Handle the query and return the result
    fn handle(&self, query: Q) -> QueryResult<R>;
}

/// Read model storage trait
pub trait ReadModelStorage<T>: Send + Sync {
    /// Get an item by ID
    fn get(&self, id: &str) -> Option<T>;

    /// Query items by criteria
    fn query(&self, criteria: &QueryCriteria) -> Vec<T>;

    /// Get all items
    fn all(&self) -> Vec<T>;
}

/// Query criteria for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCriteria {
    /// Filter conditions as key-value pairs
    pub filters: HashMap<String, serde_json::Value>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Number of results to skip
    pub offset: Option<usize>,
    /// Field to order results by
    pub order_by: Option<String>,
}

impl QueryCriteria {
    /// Create a new empty query criteria
    pub fn new() -> Self {
        Self {
            filters: HashMap::new(),
            limit: None,
            offset: None,
            order_by: None,
        }
    }

    /// Add a filter condition
    pub fn with_filter(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.filters.insert(key.into(), serde_json::to_value(value).unwrap());
        self
    }

    /// Set the result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// In-memory read model storage
#[derive(Clone)]
pub struct InMemoryReadModel<T: Clone> {
    storage: Arc<RwLock<HashMap<String, T>>>,
}

impl<T: Clone> InMemoryReadModel<T> {
    /// Create a new in-memory read model
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert an item into the read model
    pub fn insert(&self, id: String, item: T) {
        self.storage.write().unwrap().insert(id, item);
    }
}

impl<T: Clone + Send + Sync> ReadModelStorage<T> for InMemoryReadModel<T> {
    fn get(&self, id: &str) -> Option<T> {
        self.storage.read().unwrap().get(id).cloned()
    }

    fn query(&self, criteria: &QueryCriteria) -> Vec<T> {
        let storage = self.storage.read().unwrap();
        let mut results: Vec<T> = storage.values().cloned().collect();

        // Apply limit
        if let Some(limit) = criteria.limit {
            results.truncate(limit);
        }

        results
    }

    fn all(&self) -> Vec<T> {
        self.storage.read().unwrap().values().cloned().collect()
    }
}

// Person Queries and Views

/// Person view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonView {
    /// Person's unique identifier
    pub person_id: Uuid,
    /// Legal name of the person
    pub legal_name: String,
    /// Preferred name if different from legal name
    pub preferred_name: Option<String>,
    /// Primary email address
    pub email: Option<String>,
    /// Name of the person's location
    pub location_name: Option<String>,
    /// Name of the person's organization
    pub organization_name: Option<String>,
    /// List of roles the person has
    pub roles: Vec<String>,
}

/// Query to get a person by ID
#[derive(Debug, Clone)]
pub struct GetPersonById {
    /// The ID of the person to retrieve
    pub person_id: Uuid,
}

impl Query for GetPersonById {}

/// Query to find people by organization
#[derive(Debug, Clone)]
pub struct FindPeopleByOrganization {
    /// The ID of the organization to search within
    pub organization_id: Uuid,
    /// Maximum number of results to return
    pub limit: Option<usize>,
}

impl Query for FindPeopleByOrganization {}

/// Handler for person queries
pub struct PersonQueryHandler<R: ReadModelStorage<PersonView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<PersonView>> PersonQueryHandler<R> {
    /// Create a new person query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }
}

impl<R: ReadModelStorage<PersonView>> DirectQueryHandler<GetPersonById, Option<PersonView>> for PersonQueryHandler<R> {
    fn handle(&self, query: GetPersonById) -> QueryResult<Option<PersonView>> {
        Ok(self.read_model.get(&query.person_id.to_string()))
    }
}

impl<R: ReadModelStorage<PersonView>> DirectQueryHandler<FindPeopleByOrganization, Vec<PersonView>> for PersonQueryHandler<R> {
    fn handle(&self, query: FindPeopleByOrganization) -> QueryResult<Vec<PersonView>> {
        let criteria = QueryCriteria::new()
            .with_filter("organization_id", query.organization_id)
            .with_limit(query.limit.unwrap_or(100));

        Ok(self.read_model.query(&criteria))
    }
}

// CQRS QueryHandler implementations
impl<R: ReadModelStorage<PersonView>> CqrsQueryHandler<GetPersonById> for PersonQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<GetPersonById>) -> QueryAcknowledgment {
        // Execute the query
        match DirectQueryHandler::<GetPersonById, Option<PersonView>>::handle(self, envelope.query) {
            Ok(result) => {
                // Publish result to event stream
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

impl<R: ReadModelStorage<PersonView>> CqrsQueryHandler<FindPeopleByOrganization> for PersonQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<FindPeopleByOrganization>) -> QueryAcknowledgment {
        // Execute the query
        match DirectQueryHandler::<FindPeopleByOrganization, Vec<PersonView>>::handle(self, envelope.query) {
            Ok(result) => {
                // Publish result to event stream
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

// Organization Queries and Views

/// Organization view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationView {
    /// Organization's unique identifier
    pub organization_id: Uuid,
    /// Name of the organization
    pub name: String,
    /// Type of organization (Company, Department, etc.)
    pub org_type: String,
    /// Name of the parent organization if any
    pub parent_name: Option<String>,
    /// Number of members in the organization
    pub member_count: usize,
    /// Number of locations associated with the organization
    pub location_count: usize,
    /// Names of child organizational units
    pub child_units: Vec<String>,
}

/// Query to get organization hierarchy
#[derive(Debug, Clone)]
pub struct GetOrganizationHierarchy {
    /// The root organization ID to start from
    pub root_id: Uuid,
    /// Maximum depth to traverse (None for unlimited)
    pub depth: Option<usize>,
}

impl Query for GetOrganizationHierarchy {}

/// Hierarchical organization view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationHierarchyView {
    /// The organization at this level
    pub organization: OrganizationView,
    /// Child organizations in the hierarchy
    pub children: Vec<OrganizationHierarchyView>,
}

/// Handler for organization queries
pub struct OrganizationQueryHandler<R: ReadModelStorage<OrganizationView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<OrganizationView>> OrganizationQueryHandler<R> {
    /// Create a new organization query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }

    /// Build organization hierarchy recursively
    fn build_hierarchy(&self, org: OrganizationView, max_depth: usize, current_depth: usize) -> QueryResult<OrganizationHierarchyView> {
        let mut children = Vec::new();

        if current_depth < max_depth {
            // Find child organizations
            let criteria = QueryCriteria::new()
                .with_filter("parent_id", org.organization_id);

            let child_orgs = self.read_model.query(&criteria);

            for child_org in child_orgs {
                if let Ok(child_hierarchy) = self.build_hierarchy(child_org, max_depth, current_depth + 1) {
                    children.push(child_hierarchy);
                }
            }
        }

        Ok(OrganizationHierarchyView {
            organization: org,
            children,
        })
    }
}

impl<R: ReadModelStorage<OrganizationView>> DirectQueryHandler<GetOrganizationHierarchy, OrganizationHierarchyView> for OrganizationQueryHandler<R> {
    fn handle(&self, query: GetOrganizationHierarchy) -> QueryResult<OrganizationHierarchyView> {
        // Get the root organization
        let root_org = self.read_model.get(&query.root_id.to_string())
            .ok_or_else(|| "Organization not found".to_string())?;

        // Build hierarchy recursively
        let hierarchy = self.build_hierarchy(root_org, query.depth.unwrap_or(3), 0)?;

        Ok(hierarchy)
    }
}

impl<R: ReadModelStorage<OrganizationView>> CqrsQueryHandler<GetOrganizationHierarchy> for OrganizationQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<GetOrganizationHierarchy>) -> QueryAcknowledgment {
        match DirectQueryHandler::<GetOrganizationHierarchy, OrganizationHierarchyView>::handle(self, envelope.query) {
            Ok(result) => {
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

// Location Queries and Views

/// Location view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationView {
    /// Location's unique identifier
    pub location_id: Uuid,
    /// Name of the location
    pub name: String,
    /// Type of location (Physical, Virtual, etc.)
    pub location_type: String,
    /// Physical address if applicable
    pub address: Option<String>,
    /// Geographic coordinates (latitude, longitude)
    pub coordinates: Option<(f64, f64)>,
    /// Name of the parent location if any
    pub parent_location: Option<String>,
}

/// Query to find locations by type
#[derive(Debug, Clone)]
pub struct FindLocationsByType {
    /// The type of location to search for
    pub location_type: String,
    /// Maximum number of results to return
    pub limit: Option<usize>,
}

impl Query for FindLocationsByType {}

/// Handler for location queries
pub struct LocationQueryHandler<R: ReadModelStorage<LocationView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<LocationView>> LocationQueryHandler<R> {
    /// Create a new location query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }
}

impl<R: ReadModelStorage<LocationView>> DirectQueryHandler<FindLocationsByType, Vec<LocationView>> for LocationQueryHandler<R> {
    fn handle(&self, query: FindLocationsByType) -> QueryResult<Vec<LocationView>> {
        let criteria = QueryCriteria::new()
            .with_filter("location_type", query.location_type)
            .with_limit(query.limit.unwrap_or(100));

        Ok(self.read_model.query(&criteria))
    }
}

impl<R: ReadModelStorage<LocationView>> CqrsQueryHandler<FindLocationsByType> for LocationQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<FindLocationsByType>) -> QueryAcknowledgment {
        match DirectQueryHandler::<FindLocationsByType, Vec<LocationView>>::handle(self, envelope.query) {
            Ok(result) => {
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

// Policy Queries and Views

/// Policy view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyView {
    /// Policy's unique identifier
    pub policy_id: Uuid,
    /// Name of the policy
    pub name: String,
    /// Type of policy (AccessControl, DataGovernance, etc.)
    pub policy_type: String,
    /// Current status of the policy
    pub status: String,
    /// Scope where the policy applies
    pub scope: String,
    /// Name of the policy owner
    pub owner_name: Option<String>,
    /// Date when the policy becomes effective
    pub effective_date: Option<String>,
    /// Current approval status
    pub approval_status: Option<String>,
}

/// Query to find active policies
#[derive(Debug, Clone)]
pub struct FindActivePolicies {
    /// Filter by policy scope (optional)
    pub scope: Option<String>,
    /// Filter by policy type (optional)
    pub policy_type: Option<String>,
}

impl Query for FindActivePolicies {}

/// Handler for policy queries
pub struct PolicyQueryHandler<R: ReadModelStorage<PolicyView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<PolicyView>> PolicyQueryHandler<R> {
    /// Create a new policy query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }
}

impl<R: ReadModelStorage<PolicyView>> DirectQueryHandler<FindActivePolicies, Vec<PolicyView>> for PolicyQueryHandler<R> {
    fn handle(&self, query: FindActivePolicies) -> QueryResult<Vec<PolicyView>> {
        let mut criteria = QueryCriteria::new()
            .with_filter("status", "Active");

        if let Some(scope) = query.scope {
            criteria = criteria.with_filter("scope", scope);
        }

        if let Some(policy_type) = query.policy_type {
            criteria = criteria.with_filter("policy_type", policy_type);
        }

        Ok(self.read_model.query(&criteria))
    }
}

impl<R: ReadModelStorage<PolicyView>> CqrsQueryHandler<FindActivePolicies> for PolicyQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<FindActivePolicies>) -> QueryAcknowledgment {
        match DirectQueryHandler::<FindActivePolicies, Vec<PolicyView>>::handle(self, envelope.query) {
            Ok(result) => {
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

// Document Queries and Views

/// Document view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentView {
    /// Document's unique identifier
    pub document_id: Uuid,
    /// Title of the document
    pub title: String,
    /// MIME type of the document
    pub mime_type: String,
    /// Current status of the document
    pub status: String,
    /// Name of the document owner
    pub owner_name: Option<String>,
    /// Size of the document in bytes
    pub size_bytes: u64,
    /// Creation timestamp
    pub created_at: String,
    /// Tags associated with the document
    pub tags: Vec<String>,
}

/// Query to search documents
#[derive(Debug, Clone)]
pub struct SearchDocuments {
    /// Text search query
    pub query: String,
    /// Filter by tags
    pub tags: Vec<String>,
    /// Filter by MIME types
    pub mime_types: Vec<String>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
}

impl Query for SearchDocuments {}

/// Handler for document queries
pub struct DocumentQueryHandler<R: ReadModelStorage<DocumentView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<DocumentView>> DocumentQueryHandler<R> {
    /// Create a new document query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }
}

impl<R: ReadModelStorage<DocumentView>> DirectQueryHandler<SearchDocuments, Vec<DocumentView>> for DocumentQueryHandler<R> {
    fn handle(&self, query: SearchDocuments) -> QueryResult<Vec<DocumentView>> {
        let mut criteria = QueryCriteria::new();

        // Add text search filter if query is provided
        if !query.query.is_empty() {
            criteria = criteria.with_filter("text_search", query.query);
        }

        // Filter by tags
        for tag in &query.tags {
            criteria = criteria.with_filter("tag", tag.clone());
        }

        // Filter by mime types
        for mime_type in &query.mime_types {
            criteria = criteria.with_filter("mime_type", mime_type.clone());
        }

        // Apply limit
        if let Some(limit) = query.limit {
            criteria = criteria.with_limit(limit);
        }

        Ok(self.read_model.query(&criteria))
    }
}

impl<R: ReadModelStorage<DocumentView>> CqrsQueryHandler<SearchDocuments> for DocumentQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<SearchDocuments>) -> QueryAcknowledgment {
        match DirectQueryHandler::<SearchDocuments, Vec<DocumentView>>::handle(self, envelope.query) {
            Ok(result) => {
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

// Agent Queries and Views

/// Agent view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentView {
    /// Agent's unique identifier
    pub agent_id: Uuid,
    /// Name of the agent
    pub name: String,
    /// Type of agent (Human, AI, System, etc.)
    pub agent_type: String,
    /// Current status of the agent
    pub status: String,
    /// List of agent capabilities
    pub capabilities: Vec<String>,
    /// List of agent permissions
    pub permissions: Vec<String>,
    /// Name of the agent owner
    pub owner_name: Option<String>,
}

/// Query to find agents by capability
#[derive(Debug, Clone)]
pub struct FindAgentsByCapability {
    /// The capability to search for
    pub capability: String,
    /// Filter by agent status (optional)
    pub status: Option<String>,
}

impl Query for FindAgentsByCapability {}

/// Handler for agent queries
pub struct AgentQueryHandler<R: ReadModelStorage<AgentView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<AgentView>> AgentQueryHandler<R> {
    /// Create a new agent query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }
}

impl<R: ReadModelStorage<AgentView>> DirectQueryHandler<FindAgentsByCapability, Vec<AgentView>> for AgentQueryHandler<R> {
    fn handle(&self, query: FindAgentsByCapability) -> QueryResult<Vec<AgentView>> {
        let mut criteria = QueryCriteria::new()
            .with_filter("capability", query.capability);

        if let Some(status) = query.status {
            criteria = criteria.with_filter("status", status);
        }

        Ok(self.read_model.query(&criteria))
    }
}

impl<R: ReadModelStorage<AgentView>> CqrsQueryHandler<FindAgentsByCapability> for AgentQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<FindAgentsByCapability>) -> QueryAcknowledgment {
        match DirectQueryHandler::<FindAgentsByCapability, Vec<AgentView>>::handle(self, envelope.query) {
            Ok(result) => {
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

// Workflow Queries and Views

/// Workflow view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowView {
    /// Workflow's unique identifier
    pub workflow_id: Uuid,
    /// Name of the workflow definition
    pub definition_name: String,
    /// Current state of the workflow
    pub current_state: String,
    /// Current status of the workflow
    pub status: String,
    /// When the workflow was started
    pub started_at: String,
    /// Number of transitions executed
    pub transition_count: usize,
    /// Additional context data for the workflow
    pub context_data: serde_json::Value,
}

/// Query to find workflows by status
#[derive(Debug, Clone)]
pub struct FindWorkflowsByStatus {
    /// The status to search for
    pub status: String,
    /// Maximum number of results to return
    pub limit: Option<usize>,
}

impl Query for FindWorkflowsByStatus {}

/// Handler for workflow queries
pub struct WorkflowQueryHandler<R: ReadModelStorage<WorkflowView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<WorkflowView>> WorkflowQueryHandler<R> {
    /// Create a new workflow query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }
}

impl<R: ReadModelStorage<WorkflowView>> DirectQueryHandler<FindWorkflowsByStatus, Vec<WorkflowView>> for WorkflowQueryHandler<R> {
    fn handle(&self, query: FindWorkflowsByStatus) -> QueryResult<Vec<WorkflowView>> {
        let criteria = QueryCriteria::new()
            .with_filter("status", query.status)
            .with_limit(query.limit.unwrap_or(100));

        Ok(self.read_model.query(&criteria))
    }
}

impl<R: ReadModelStorage<WorkflowView>> CqrsQueryHandler<FindWorkflowsByStatus> for WorkflowQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<FindWorkflowsByStatus>) -> QueryAcknowledgment {
        match DirectQueryHandler::<FindWorkflowsByStatus, Vec<WorkflowView>>::handle(self, envelope.query) {
            Ok(result) => {
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_query_handler() {
        // Setup
        let read_model = InMemoryReadModel::<PersonView>::new();
        let handler = PersonQueryHandler::new(read_model.clone(), Arc::new(MockEventPublisher));

        // Insert test data
        let person_view = PersonView {
            person_id: Uuid::new_v4(),
            legal_name: "John Doe".to_string(),
            preferred_name: Some("John".to_string()),
            email: Some("john@example.com".to_string()),
            location_name: Some("New York".to_string()),
            organization_name: Some("Tech Corp".to_string()),
            roles: vec!["Developer".to_string()],
        };

        read_model.insert(person_view.person_id.to_string(), person_view.clone());

        // Test query
        let query = GetPersonById {
            person_id: person_view.person_id,
        };

        let result = DirectQueryHandler::<GetPersonById, Option<PersonView>>::handle(&handler, query).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().legal_name, "John Doe");
    }

    #[test]
    fn test_find_people_by_organization() {
        // Setup
        let read_model = InMemoryReadModel::<PersonView>::new();
        let handler = PersonQueryHandler::new(read_model.clone(), Arc::new(MockEventPublisher));

        let org_id = Uuid::new_v4();

        // Insert test data
        for i in 0..3 {
            let person_view = PersonView {
                person_id: Uuid::new_v4(),
                legal_name: format!("Person {}", i),
                preferred_name: None,
                email: Some(format!("person{}@example.com", i)),
                location_name: None,
                organization_name: Some("Tech Corp".to_string()),
                roles: vec!["Employee".to_string()],
            };

            read_model.insert(person_view.person_id.to_string(), person_view);
        }

        // Test query
        let query = FindPeopleByOrganization {
            organization_id: org_id,
            limit: Some(10),
        };

        let result = DirectQueryHandler::<FindPeopleByOrganization, Vec<PersonView>>::handle(&handler, query).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_policy_active_query() {
        // Setup
        let read_model = InMemoryReadModel::<PolicyView>::new();
        let handler = PolicyQueryHandler::new(read_model.clone(), Arc::new(MockEventPublisher));

        // Insert test data
        let policy_view = PolicyView {
            policy_id: Uuid::new_v4(),
            name: "Data Access Policy".to_string(),
            policy_type: "AccessControl".to_string(),
            status: "Active".to_string(),
            scope: "Global".to_string(),
            owner_name: Some("Admin".to_string()),
            effective_date: Some("2025-01-01".to_string()),
            approval_status: Some("Approved".to_string()),
        };

        read_model.insert(policy_view.policy_id.to_string(), policy_view);

        // Test query
        let query = FindActivePolicies {
            scope: Some("Global".to_string()),
            policy_type: None,
        };

        let result = DirectQueryHandler::<FindActivePolicies, Vec<PolicyView>>::handle(&handler, query).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Data Access Policy");
    }

    #[test]
    fn test_agent_capability_query() {
        // Setup
        let read_model = InMemoryReadModel::<AgentView>::new();
        let handler = AgentQueryHandler::new(read_model.clone(), Arc::new(MockEventPublisher));

        // Insert test data
        let agent_view = AgentView {
            agent_id: Uuid::new_v4(),
            name: "AI Assistant".to_string(),
            agent_type: "Assistant".to_string(),
            status: "Active".to_string(),
            capabilities: vec!["text-generation".to_string(), "code-analysis".to_string()],
            permissions: vec!["read".to_string()],
            owner_name: Some("System".to_string()),
        };

        read_model.insert(agent_view.agent_id.to_string(), agent_view);

        // Test query
        let query = FindAgentsByCapability {
            capability: "text-generation".to_string(),
            status: Some("Active".to_string()),
        };

        let result = DirectQueryHandler::<FindAgentsByCapability, Vec<AgentView>>::handle(&handler, query).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "AI Assistant");
    }

    #[test]
    fn test_workflow_status_query() {
        // Setup
        let read_model = InMemoryReadModel::<WorkflowView>::new();
        let handler = WorkflowQueryHandler::new(read_model.clone(), Arc::new(MockEventPublisher));

        // Insert test data
        let workflow_view = WorkflowView {
            workflow_id: Uuid::new_v4(),
            definition_name: "Order Processing".to_string(),
            current_state: "Processing".to_string(),
            status: "Active".to_string(),
            started_at: "2025-01-01T00:00:00Z".to_string(),
            transition_count: 3,
            context_data: serde_json::json!({"order_id": "12345"}),
        };

        read_model.insert(workflow_view.workflow_id.to_string(), workflow_view);

        // Test query
        let query = FindWorkflowsByStatus {
            status: "Active".to_string(),
            limit: Some(10),
        };

        let result = DirectQueryHandler::<FindWorkflowsByStatus, Vec<WorkflowView>>::handle(&handler, query).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].definition_name, "Order Processing");
    }

    #[test]
    fn test_location_type_query() {
        // Setup
        let read_model = InMemoryReadModel::<LocationView>::new();
        let handler = LocationQueryHandler::new(read_model.clone(), Arc::new(MockEventPublisher));

        // Insert test data
        let location_view = LocationView {
            location_id: Uuid::new_v4(),
            name: "Main Office".to_string(),
            location_type: "Physical".to_string(),
            address: Some("123 Main St".to_string()),
            coordinates: Some((40.7128, -74.0060)),
            parent_location: None,
        };

        read_model.insert(location_view.location_id.to_string(), location_view);

        // Test query
        let query = FindLocationsByType {
            location_type: "Physical".to_string(),
            limit: Some(10),
        };

        let result = DirectQueryHandler::<FindLocationsByType, Vec<LocationView>>::handle(&handler, query).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Main Office");
    }
}
