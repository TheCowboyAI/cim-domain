/// User Story 4: Data Analyst - Building Projections and Queries
/// 
/// As a Data Analyst, I want to build projections from event streams and
/// create complex queries, so that I can provide insights and analytics
/// to business stakeholders.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Core types for this example
#[derive(Debug, Clone)]
pub enum DomainError {
    ValidationError(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent {
    pub fn new(aggregate_id: String, payload: serde_json::Value) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            aggregate_id,
            event_type: "DomainEvent".into(),
            payload,
            timestamp: chrono::Utc::now(),
        }
    }
}

// Event store trait
#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, events: Vec<DomainEvent>) -> Result<(), DomainError>;
}

#[derive(Debug, Clone)]
pub struct InMemoryEventStore;

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, _events: Vec<DomainEvent>) -> Result<(), DomainError> {
        Ok(())
    }
}

// Projection definitions
#[derive(Debug, Clone)]
pub struct ProjectionDefinition {
    pub name: String,
    pub source_streams: Vec<String>,
    pub event_handlers: HashMap<String, ProjectionHandler>,
    pub query_model: QueryModel,
}

#[derive(Debug, Clone)]
pub enum ProjectionHandler {
    Increment { field: String },
    Decrement { field: String },
    Set { field: String, value_path: String },
    Append { field: String, value_path: String },
    Custom { function: String },
}

#[derive(Debug, Clone)]
pub struct QueryModel {
    pub fields: Vec<FieldDefinition>,
    pub indexes: Vec<IndexDefinition>,
    pub aggregations: Vec<AggregationDefinition>,
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub default_value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Date,
    Array(Box<FieldType>),
    Object,
}

#[derive(Debug, Clone)]
pub struct IndexDefinition {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub struct AggregationDefinition {
    pub name: String,
    pub aggregation_type: AggregationType,
    pub field: String,
    pub group_by: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum AggregationType {
    Sum,
    Average,
    Count,
    Min,
    Max,
    Percentile(f64),
}

// Analytics queries
#[derive(Debug, Clone)]
pub struct AnalyticsQuery {
    pub name: String,
    pub projection: String,
    pub filters: Vec<QueryFilter>,
    pub aggregations: Vec<String>,
    pub group_by: Vec<String>,
    pub order_by: Vec<OrderBy>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    In,
    Between,
    Contains,
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone)]
pub enum SortDirection {
    Ascending,
    Descending,
}

// Projection results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionResult {
    pub id: String,
    pub data: serde_json::Value,
    pub version: u64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

// Data Analyst workspace
pub struct DataAnalystWorkspace {
    projections: Arc<RwLock<HashMap<String, ProjectionDefinition>>>,
    projection_data: Arc<RwLock<HashMap<String, Vec<ProjectionResult>>>>,
    saved_queries: Arc<RwLock<HashMap<String, AnalyticsQuery>>>,
    event_store: Box<dyn EventStore>,
}

impl DataAnalystWorkspace {
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            projections: Arc::new(RwLock::new(HashMap::new())),
            projection_data: Arc::new(RwLock::new(HashMap::new())),
            saved_queries: Arc::new(RwLock::new(HashMap::new())),
            event_store,
        }
    }

    pub async fn create_projection(&self, definition: ProjectionDefinition) -> Result<(), DomainError> {
        println!("Creating projection: {}", definition.name);
        
        // Validate projection definition
        if definition.source_streams.is_empty() {
            return Err(DomainError::ValidationError("Projection must have at least one source stream".into()));
        }

        // Store projection definition
        let mut projections = self.projections.write().await;
        projections.insert(definition.name.clone(), definition.clone());

        // Initialize projection data storage
        let mut data = self.projection_data.write().await;
        data.insert(definition.name.clone(), Vec::new());

        Ok(())
    }

    pub async fn process_event(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let projections = self.projections.read().await;
        
        for (name, definition) in projections.iter() {
            // Check if this projection handles this event type
            if let Some(handler) = definition.event_handlers.get(&event.aggregate_id) {
                self.update_projection(name, event, handler).await?;
            }
        }

        Ok(())
    }

    async fn update_projection(&self, projection_name: &str, event: &DomainEvent, handler: &ProjectionHandler) -> Result<(), DomainError> {
        let mut data = self.projection_data.write().await;
        let projection_data = data.get_mut(projection_name).unwrap();

        match handler {
            ProjectionHandler::Increment { field } => {
                // Find or create projection entry
                let entry = self.find_or_create_entry(projection_data, event).await?;
                if let Some(value) = entry.data.get_mut(field) {
                    if let Some(num) = value.as_f64() {
                        *value = serde_json::json!(num + 1.0);
                    }
                } else {
                    entry.data[field] = serde_json::json!(1);
                }
                entry.version += 1;
                entry.last_updated = chrono::Utc::now();
            }
            ProjectionHandler::Set { field, value_path } => {
                let entry = self.find_or_create_entry(projection_data, event).await?;
                if let Some(value) = event.payload.get(value_path) {
                    entry.data[field] = value.clone();
                    entry.version += 1;
                    entry.last_updated = chrono::Utc::now();
                }
            }
            ProjectionHandler::Append { field, value_path } => {
                let entry = self.find_or_create_entry(projection_data, event).await?;
                if let Some(value) = event.payload.get(value_path) {
                    if let Some(arr) = entry.data[field].as_array_mut() {
                        arr.push(value.clone());
                    } else {
                        entry.data[field] = serde_json::json!([value]);
                    }
                    entry.version += 1;
                    entry.last_updated = chrono::Utc::now();
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn find_or_create_entry<'a>(&self, projection_data: &'a mut Vec<ProjectionResult>, event: &DomainEvent) -> Result<&'a mut ProjectionResult, DomainError> {
        // For demo, use aggregate_id as entry id
        let id = event.aggregate_id.clone();
        
        let position = projection_data.iter().position(|e| e.id == id);
        
        if let Some(pos) = position {
            Ok(&mut projection_data[pos])
        } else {
            projection_data.push(ProjectionResult {
                id: id.clone(),
                data: serde_json::json!({}),
                version: 0,
                last_updated: chrono::Utc::now(),
            });
            Ok(projection_data.last_mut().unwrap())
        }
    }

    pub async fn execute_query(&self, query: &AnalyticsQuery) -> Result<Vec<serde_json::Value>, DomainError> {
        let data = self.projection_data.read().await;
        
        let projection_data = data.get(&query.projection)
            .ok_or_else(|| DomainError::ValidationError(format!("Projection '{}' not found", query.projection)))?;

        // Apply filters
        let mut results: Vec<&ProjectionResult> = projection_data.iter()
            .filter(|entry| self.apply_filters(entry, &query.filters))
            .collect();

        // Apply sorting
        if !query.order_by.is_empty() {
            results.sort_by(|a, b| {
                for order in &query.order_by {
                    let a_val = a.data.get(&order.field);
                    let b_val = b.data.get(&order.field);
                    
                    match (a_val, b_val, &order.direction) {
                        (Some(a), Some(b), SortDirection::Ascending) => {
                            if a != b {
                                return a.to_string().cmp(&b.to_string());
                            }
                        }
                        (Some(a), Some(b), SortDirection::Descending) => {
                            if a != b {
                                return b.to_string().cmp(&a.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        // Convert to JSON values
        Ok(results.into_iter().map(|r| r.data.clone()).collect())
    }

    fn apply_filters(&self, entry: &ProjectionResult, filters: &[QueryFilter]) -> bool {
        filters.iter().all(|filter| {
            if let Some(value) = entry.data.get(&filter.field) {
                match &filter.operator {
                    FilterOperator::Equals => value == &filter.value,
                    FilterOperator::NotEquals => value != &filter.value,
                    FilterOperator::GreaterThan => {
                        if let (Some(a), Some(b)) = (value.as_f64(), filter.value.as_f64()) {
                            a > b
                        } else {
                            false
                        }
                    }
                    FilterOperator::LessThan => {
                        if let (Some(a), Some(b)) = (value.as_f64(), filter.value.as_f64()) {
                            a < b
                        } else {
                            false
                        }
                    }
                    FilterOperator::Contains => {
                        if let (Some(str_val), Some(search)) = (value.as_str(), filter.value.as_str()) {
                            str_val.contains(search)
                        } else {
                            false
                        }
                    }
                    _ => true,
                }
            } else {
                false
            }
        })
    }

    pub async fn save_query(&self, query: AnalyticsQuery) -> Result<(), DomainError> {
        let mut queries = self.saved_queries.write().await;
        let query_name = query.name.clone();
        queries.insert(query_name.clone(), query);
        
        // Log to event store
        let event = DomainEvent::new(
            "analytics".into(),
            serde_json::json!({
                "type": "QuerySaved",
                "query_name": query_name
            }),
        );
        self.event_store.append(vec![event]).await?;
        
        Ok(())
    }

    pub async fn generate_report(&self, query_name: &str) -> Result<AnalyticsReport, DomainError> {
        let queries = self.saved_queries.read().await;
        let query = queries.get(query_name)
            .ok_or_else(|| DomainError::ValidationError(format!("Query '{}' not found", query_name)))?;

        let results = self.execute_query(query).await?;

        // Calculate aggregations
        let mut aggregations = HashMap::new();
        
        for agg_name in &query.aggregations {
            let projections = self.projections.read().await;
            if let Some(projection) = projections.get(&query.projection) {
                if let Some(agg_def) = projection.query_model.aggregations.iter().find(|a| &a.name == agg_name) {
                    let agg_result = self.calculate_aggregation(&results, &agg_def)?;
                    aggregations.insert(agg_name.clone(), agg_result);
                }
            }
        }

        Ok(AnalyticsReport {
            query_name: query_name.to_string(),
            execution_time: chrono::Utc::now(),
            row_count: results.len(),
            results,
            aggregations,
        })
    }

    fn calculate_aggregation(&self, results: &[serde_json::Value], definition: &AggregationDefinition) -> Result<serde_json::Value, DomainError> {
        match &definition.aggregation_type {
            AggregationType::Count => Ok(serde_json::json!(results.len())),
            AggregationType::Sum => {
                let sum: f64 = results.iter()
                    .filter_map(|r| r.get(&definition.field)?.as_f64())
                    .sum();
                Ok(serde_json::json!(sum))
            }
            AggregationType::Average => {
                let values: Vec<f64> = results.iter()
                    .filter_map(|r| r.get(&definition.field)?.as_f64())
                    .collect();
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                Ok(serde_json::json!(avg))
            }
            AggregationType::Min => {
                let min = results.iter()
                    .filter_map(|r| r.get(&definition.field)?.as_f64())
                    .min_by(|a, b| a.partial_cmp(b).unwrap());
                Ok(serde_json::json!(min))
            }
            AggregationType::Max => {
                let max = results.iter()
                    .filter_map(|r| r.get(&definition.field)?.as_f64())
                    .max_by(|a, b| a.partial_cmp(b).unwrap());
                Ok(serde_json::json!(max))
            }
            _ => Ok(serde_json::json!(null)),
        }
    }
}

#[derive(Debug)]
pub struct AnalyticsReport {
    pub query_name: String,
    pub execution_time: chrono::DateTime<chrono::Utc>,
    pub row_count: usize,
    pub results: Vec<serde_json::Value>,
    pub aggregations: HashMap<String, serde_json::Value>,
}

// Demo: E-commerce analytics scenario
pub async fn demonstrate_analytics_workflow(workspace: &DataAnalystWorkspace) {
    println!("\nDemonstrating Analytics Workflow");
    println!("================================\n");

    // Create order analytics projection
    let order_projection = ProjectionDefinition {
        name: "order_analytics".into(),
        source_streams: vec!["orders".into()],
        event_handlers: vec![
            ("order.created".into(), ProjectionHandler::Increment { field: "total_orders".into() }),
            ("order.created".into(), ProjectionHandler::Set { 
                field: "last_order_amount".into(), 
                value_path: "total_amount".into() 
            }),
            ("order.completed".into(), ProjectionHandler::Increment { field: "completed_orders".into() }),
            ("order.cancelled".into(), ProjectionHandler::Increment { field: "cancelled_orders".into() }),
        ].into_iter().collect(),
        query_model: QueryModel {
            fields: vec![
                FieldDefinition {
                    name: "total_orders".into(),
                    field_type: FieldType::Number,
                    default_value: serde_json::json!(0),
                },
                FieldDefinition {
                    name: "completed_orders".into(),
                    field_type: FieldType::Number,
                    default_value: serde_json::json!(0),
                },
                FieldDefinition {
                    name: "cancelled_orders".into(),
                    field_type: FieldType::Number,
                    default_value: serde_json::json!(0),
                },
            ],
            indexes: vec![],
            aggregations: vec![
                AggregationDefinition {
                    name: "completion_rate".into(),
                    aggregation_type: AggregationType::Average,
                    field: "completed_orders".into(),
                    group_by: None,
                },
            ],
        },
    };

    workspace.create_projection(order_projection).await.unwrap();

    // Create customer behavior projection
    let customer_projection = ProjectionDefinition {
        name: "customer_behavior".into(),
        source_streams: vec!["orders".into(), "customers".into()],
        event_handlers: vec![
            ("order.created".into(), ProjectionHandler::Set {
                field: "customer_id".into(),
                value_path: "customer_id".into(),
            }),
            ("order.created".into(), ProjectionHandler::Append {
                field: "order_history".into(),
                value_path: "order_id".into(),
            }),
        ].into_iter().collect(),
        query_model: QueryModel {
            fields: vec![
                FieldDefinition {
                    name: "customer_id".into(),
                    field_type: FieldType::String,
                    default_value: serde_json::json!(""),
                },
                FieldDefinition {
                    name: "order_history".into(),
                    field_type: FieldType::Array(Box::new(FieldType::String)),
                    default_value: serde_json::json!([]),
                },
            ],
            indexes: vec![
                IndexDefinition {
                    name: "customer_id_idx".into(),
                    fields: vec!["customer_id".into()],
                    unique: true,
                },
            ],
            aggregations: vec![],
        },
    };

    workspace.create_projection(customer_projection).await.unwrap();

    // Process some events
    println!("Processing events...");
    let events = vec![
        DomainEvent::new(
            "order.created".into(),
            serde_json::json!({
                "order_id": "ORD-001",
                "customer_id": "CUST-123",
                "total_amount": 250.00
            }),
        ),
        DomainEvent::new(
            "order.created".into(),
            serde_json::json!({
                "order_id": "ORD-002",
                "customer_id": "CUST-124",
                "total_amount": 1500.00
            }),
        ),
        DomainEvent::new(
            "order.completed".into(),
            serde_json::json!({
                "order_id": "ORD-001"
            }),
        ),
        DomainEvent::new(
            "order.created".into(),
            serde_json::json!({
                "order_id": "ORD-003",
                "customer_id": "CUST-123",
                "total_amount": 75.00
            }),
        ),
        DomainEvent::new(
            "order.cancelled".into(),
            serde_json::json!({
                "order_id": "ORD-002"
            }),
        ),
    ];

    for event in &events {
        workspace.process_event(event).await.unwrap();
    }

    // Create and save queries
    let high_value_orders_query = AnalyticsQuery {
        name: "high_value_orders".into(),
        projection: "order_analytics".into(),
        filters: vec![
            QueryFilter {
                field: "last_order_amount".into(),
                operator: FilterOperator::GreaterThan,
                value: serde_json::json!(100),
            },
        ],
        aggregations: vec!["completion_rate".into()],
        group_by: vec![],
        order_by: vec![
            OrderBy {
                field: "last_order_amount".into(),
                direction: SortDirection::Descending,
            },
        ],
        limit: Some(10),
    };

    workspace.save_query(high_value_orders_query).await.unwrap();

    let customer_orders_query = AnalyticsQuery {
        name: "customer_order_frequency".into(),
        projection: "customer_behavior".into(),
        filters: vec![],
        aggregations: vec![],
        group_by: vec!["customer_id".into()],
        order_by: vec![],
        limit: None,
    };

    workspace.save_query(customer_orders_query).await.unwrap();

    // Generate reports
    println!("\nGenerating Analytics Reports:");
    println!("=============================\n");

    let report = workspace.generate_report("high_value_orders").await.unwrap();
    println!("Report: {}", report.query_name);
    println!("Execution Time: {}", report.execution_time.format("%Y-%m-%d %H:%M:%S"));
    println!("Rows Returned: {}", report.row_count);
    println!("\nResults:");
    for (i, result) in report.results.iter().enumerate() {
        println!("  {}: {}", i + 1, serde_json::to_string_pretty(result).unwrap());
    }
    println!("\nAggregations:");
    for (name, value) in &report.aggregations {
        println!("  {}: {}", name, value);
    }

    // Show projection data
    println!("\n\nCurrent Projection Data:");
    println!("========================");
    let data = workspace.projection_data.read().await;
    for (projection_name, entries) in data.iter() {
        println!("\n{}:", projection_name);
        for entry in entries {
            println!("  ID: {}, Version: {}, Data: {}", 
                entry.id, 
                entry.version,
                serde_json::to_string(&entry.data).unwrap()
            );
        }
    }
}

#[tokio::main]
async fn main() {
    println!("User Story 4: Data Analyst Demo");
    println!("===============================\n");

    let event_store = Box::new(InMemoryEventStore::new());
    let workspace = DataAnalystWorkspace::new(event_store);

    // Run the demonstration
    demonstrate_analytics_workflow(&workspace).await;

    println!("\n\nKey Features Demonstrated:");
    println!("✓ Projection creation from event streams");
    println!("✓ Event processing and projection updates");
    println!("✓ Complex query building with filters");
    println!("✓ Aggregation calculations");
    println!("✓ Saved queries for reuse");
    println!("✓ Analytics report generation");
    println!("✓ Real-time data analysis capabilities");
}