/// User Story 1: Component Developer - Building a Reusable UI Component
/// 
/// As a Component Developer, I want to build a reusable search component
/// that emits domain events and maintains its own state, so that it can
/// be integrated into different parts of the application.

use serde::{Deserialize, Serialize};

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

impl From<serde_json::Error> for DomainError {
    fn from(err: serde_json::Error) -> Self {
        DomainError::ValidationError(err.to_string())
    }
}

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
pub struct InMemoryEventStore {
    events: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<DomainEvent>>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, events: Vec<DomainEvent>) -> Result<(), DomainError> {
        let mut store = self.events.write().await;
        for event in events {
            let entry = store.entry(event.aggregate_id.clone()).or_insert_with(Vec::new);
            entry.push(event);
        }
        Ok(())
    }
}

pub type ComponentId = String;
pub type DomainContext = String;

// Define the search component's domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchEvent {
    QuerySubmitted { query: String, timestamp: i64 },
    ResultsReceived { count: usize, duration_ms: u64 },
    FilterApplied { filter_type: String, value: String },
    SearchCleared,
}

// Define the component's state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchState {
    current_query: Option<String>,
    active_filters: Vec<(String, String)>,
    last_result_count: usize,
    search_history: Vec<String>,
}

// Implement the search component as a domain component
pub struct SearchComponent {
    id: ComponentId,
    state: SearchState,
    event_store: Box<dyn EventStore>,
}

impl SearchComponent {
    pub fn new(id: ComponentId, event_store: Box<dyn EventStore>) -> Self {
        Self {
            id,
            state: SearchState::default(),
            event_store,
        }
    }

    pub async fn submit_search(&mut self, query: String) -> Result<(), DomainError> {
        // Validate the query
        if query.trim().is_empty() {
            return Err(DomainError::ValidationError("Query cannot be empty".into()));
        }

        // Create and emit event
        let event = SearchEvent::QuerySubmitted {
            query: query.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        // Update internal state
        self.state.current_query = Some(query.clone());
        self.state.search_history.push(query);

        // Emit to event stream
        self.emit_event(event).await?;

        Ok(())
    }

    pub async fn apply_filter(&mut self, filter_type: String, value: String) -> Result<(), DomainError> {
        let event = SearchEvent::FilterApplied {
            filter_type: filter_type.clone(),
            value: value.clone(),
        };

        self.state.active_filters.push((filter_type, value));
        self.emit_event(event).await?;

        Ok(())
    }

    pub async fn record_results(&mut self, count: usize, duration_ms: u64) -> Result<(), DomainError> {
        let event = SearchEvent::ResultsReceived { count, duration_ms };
        
        self.state.last_result_count = count;
        self.emit_event(event).await?;

        Ok(())
    }

    pub async fn clear_search(&mut self) -> Result<(), DomainError> {
        self.state.current_query = None;
        self.state.active_filters.clear();
        self.state.last_result_count = 0;

        self.emit_event(SearchEvent::SearchCleared).await?;
        Ok(())
    }

    async fn emit_event(&self, event: SearchEvent) -> Result<(), DomainError> {
        let domain_event = DomainEvent::new(
            self.id.to_string(),
            serde_json::to_value(&event)?,
        );

        self.event_store.append(vec![domain_event]).await?;
        Ok(())
    }

    pub fn get_state(&self) -> &SearchState {
        &self.state
    }

    pub fn get_search_suggestions(&self) -> Vec<String> {
        // Simple suggestion algorithm based on history
        self.state.search_history
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect()
    }
}

// Create a reusable search widget that can be embedded in different contexts
pub struct SearchWidget {
    component: SearchComponent,
    context: DomainContext,
}

impl SearchWidget {
    pub fn new(context: DomainContext, event_store: Box<dyn EventStore>) -> Self {
        let component_id = ComponentId::new();
        Self {
            component: SearchComponent::new(component_id, event_store),
            context,
        }
    }

    pub fn get_context(&self) -> &str {
        &self.context
    }

    pub async fn render(&self) -> String {
        let state = self.component.get_state();
        let suggestions = self.component.get_search_suggestions();

        format!(
            r#"
SearchWidget {{
    current_query: {:?},
    active_filters: {} filters,
    last_results: {} items,
    suggestions: {:?}
}}
"#,
            state.current_query,
            state.active_filters.len(),
            state.last_result_count,
            suggestions
        )
    }

    pub async fn handle_user_input(&mut self, input: UserInput) -> Result<(), DomainError> {
        match input {
            UserInput::Search(query) => {
                self.component.submit_search(query).await?;
            }
            UserInput::Filter { filter_type, value } => {
                self.component.apply_filter(filter_type, value).await?;
            }
            UserInput::Clear => {
                self.component.clear_search().await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum UserInput {
    Search(String),
    Filter { filter_type: String, value: String },
    Clear,
}

// Demonstrate composition with other components
pub struct ProductSearchPage {
    search_widget: SearchWidget,
    product_list: ProductListComponent,
}

pub struct ProductListComponent {
    products: Vec<Product>,
}

#[derive(Debug)]
pub struct Product {
    id: String,
    name: String,
    price: f64,
}

impl ProductSearchPage {
    pub fn new(context: DomainContext, event_store: Box<dyn EventStore>) -> Self {
        Self {
            search_widget: SearchWidget::new(context, event_store),
            product_list: ProductListComponent { products: vec![] },
        }
    }

    pub async fn handle_search(&mut self, query: String) -> Result<(), DomainError> {
        // Use the search widget
        self.search_widget.handle_user_input(UserInput::Search(query.clone())).await?;
        
        // Simulate product search
        let products = self.search_products(&query).await?;
        self.product_list.products = products;

        // Record results in the search component
        let duration_ms = 150; // Simulated search time
        self.search_widget.component.record_results(
            self.product_list.products.len(),
            duration_ms
        ).await?;

        Ok(())
    }

    async fn search_products(&self, query: &str) -> Result<Vec<Product>, DomainError> {
        // Simulate product search with IDs
        Ok(vec![
            Product { 
                id: format!("PRD-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap()),
                name: format!("{} Pro", query), 
                price: 99.99 
            },
            Product { 
                id: format!("PRD-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap()),
                name: format!("{} Lite", query), 
                price: 49.99 
            },
        ])
    }

    pub fn get_products(&self) -> &[Product] {
        &self.product_list.products
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_component_lifecycle() {
        let event_store = Box::new(InMemoryEventStore::new());
        let mut component = SearchComponent::new(ComponentId::new(), event_store);

        // Submit a search
        component.submit_search("laptop".into()).await.unwrap();
        assert_eq!(component.get_state().current_query, Some("laptop".into()));

        // Apply filters
        component.apply_filter("brand".into(), "Dell".into()).await.unwrap();
        component.apply_filter("price".into(), "under-1000".into()).await.unwrap();
        assert_eq!(component.get_state().active_filters.len(), 2);

        // Record results
        component.record_results(25, 120).await.unwrap();
        assert_eq!(component.get_state().last_result_count, 25);

        // Clear search
        component.clear_search().await.unwrap();
        assert!(component.get_state().current_query.is_none());
        assert!(component.get_state().active_filters.is_empty());
    }

    #[tokio::test]
    async fn test_search_suggestions() {
        let event_store = Box::new(InMemoryEventStore::new());
        let mut component = SearchComponent::new(ComponentId::new(), event_store);

        // Build search history
        for query in ["laptop", "gaming laptop", "laptop dell", "laptop under 1000"] {
            component.submit_search(query.into()).await.unwrap();
        }

        let suggestions = component.get_search_suggestions();
        assert_eq!(suggestions.len(), 4);
        assert_eq!(suggestions[0], "laptop under 1000"); // Most recent first
    }

    #[tokio::test]
    async fn test_widget_integration() {
        let context = "ui-components".to_string();
        let event_store = Box::new(InMemoryEventStore::new());
        let mut widget = SearchWidget::new(context, event_store);

        // Test rendering
        let output = widget.render().await;
        assert!(output.contains("SearchWidget"));

        // Test user interactions
        widget.handle_user_input(UserInput::Search("test query".into())).await.unwrap();
        widget.handle_user_input(UserInput::Filter {
            filter_type: "category".into(),
            value: "electronics".into()
        }).await.unwrap();

        let output = widget.render().await;
        assert!(output.contains("1 filters"));
        
        // Test context access
        assert_eq!(widget.get_context(), "ui-components");
    }
}

#[tokio::main]
async fn main() {
    println!("User Story 1: Component Developer Demo");
    println!("=====================================\n");

    // Create a search widget
    let context = "product-search".to_string();
    let event_store = Box::new(InMemoryEventStore::new());
    let mut page = ProductSearchPage::new(context, event_store);

    // Demonstrate component usage
    println!("1. Initial state:");
    println!("{}", page.search_widget.render().await);

    // Perform a search
    println!("\n2. Searching for 'gaming laptop':");
    page.handle_search("gaming laptop".into()).await.unwrap();
    println!("{}", page.search_widget.render().await);

    // Apply filters
    println!("\n3. Applying filters:");
    page.search_widget.handle_user_input(UserInput::Filter {
        filter_type: "brand".into(),
        value: "ASUS".into(),
    }).await.unwrap();
    page.search_widget.handle_user_input(UserInput::Filter {
        filter_type: "price_range".into(),
        value: "1000-2000".into(),
    }).await.unwrap();
    println!("{}", page.search_widget.render().await);

    // Show found products
    println!("\n4. Found products:");
    for product in page.get_products() {
        println!("  - [{}] {} (${:.2})", product.id, product.name, product.price);
    }
    
    // Show context usage
    println!("\nSearch widget context: {}", page.search_widget.get_context());

    // Clear search
    println!("\n5. Clearing search:");
    page.search_widget.handle_user_input(UserInput::Clear).await.unwrap();
    println!("{}", page.search_widget.render().await);

    println!("\nDemo completed! The search component:");
    println!("✓ Maintains its own state");
    println!("✓ Emits domain events for all interactions");
    println!("✓ Can be reused in different contexts");
    println!("✓ Provides search suggestions based on history");
}