// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating query handling patterns
//!
//! This example shows:
//! - Implementing queries with the Query trait
//! - Creating query handlers that return acknowledgments
//! - Using read models for query results
//! - Query criteria and filtering

use cim_domain::{
    // Query support
    DirectQueryHandler,
    // Identifiers
    IdType,
    InMemoryReadModel,
    // CQRS
    Query,
    QueryCriteria,

    QueryEnvelope,
    QueryHandler,
    QueryResponse,

    QueryResult,
    ReadModelStorage,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Example read model: ProductView
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductView {
    id: String,
    name: String,
    description: String,
    price: f64,
    stock: u32,
    category: String,
    tags: Vec<String>,
}

/// Query for finding products
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ProductQuery {
    GetById {
        id: String,
    },
    SearchByName {
        name_contains: String,
    },
    FindByCategory {
        category: String,
        min_price: Option<f64>,
        max_price: Option<f64>,
    },
    GetInStock {
        min_stock: u32,
    },
}

impl Query for ProductQuery {
    // Queries don't need additional methods
}

/// Direct query handler for products (returns data)
struct ProductDirectQueryHandler {
    storage: InMemoryReadModel<ProductView>,
}

impl ProductDirectQueryHandler {
    fn new() -> Self {
        let storage = InMemoryReadModel::new();

        // Add some sample products
        let products = vec![
            ProductView {
                id: "prod-1".to_string(),
                name: "Laptop Pro".to_string(),
                description: "High-performance laptop".to_string(),
                price: 1299.99,
                stock: 15,
                category: "Electronics".to_string(),
                tags: vec!["computer".to_string(), "portable".to_string()],
            },
            ProductView {
                id: "prod-2".to_string(),
                name: "Wireless Mouse".to_string(),
                description: "Ergonomic wireless mouse".to_string(),
                price: 49.99,
                stock: 50,
                category: "Electronics".to_string(),
                tags: vec!["accessory".to_string(), "wireless".to_string()],
            },
            ProductView {
                id: "prod-3".to_string(),
                name: "Office Chair".to_string(),
                description: "Comfortable office chair".to_string(),
                price: 299.99,
                stock: 8,
                category: "Furniture".to_string(),
                tags: vec!["office".to_string(), "seating".to_string()],
            },
            ProductView {
                id: "prod-4".to_string(),
                name: "Standing Desk".to_string(),
                description: "Adjustable standing desk".to_string(),
                price: 599.99,
                stock: 5,
                category: "Furniture".to_string(),
                tags: vec!["office".to_string(), "desk".to_string()],
            },
        ];

        for product in products {
            storage.insert(product.id.clone(), product);
        }

        Self { storage }
    }
}

impl DirectQueryHandler<ProductQuery, Vec<ProductView>> for ProductDirectQueryHandler {
    fn handle(&self, query: ProductQuery) -> QueryResult<Vec<ProductView>> {
        match query {
            ProductQuery::GetById { id } => {
                Ok(self.storage.get(&id).map(|p| vec![p]).unwrap_or_default())
            }

            ProductQuery::SearchByName { name_contains } => {
                let name_lower = name_contains.to_lowercase();
                Ok(self
                    .storage
                    .all()
                    .into_iter()
                    .filter(|p| p.name.to_lowercase().contains(&name_lower))
                    .collect())
            }

            ProductQuery::FindByCategory {
                category,
                min_price,
                max_price,
            } => Ok(self
                .storage
                .all()
                .into_iter()
                .filter(|p| {
                    p.category == category
                        && min_price.map_or(true, |min| p.price >= min)
                        && max_price.map_or(true, |max| p.price <= max)
                })
                .collect()),

            ProductQuery::GetInStock { min_stock } => Ok(self
                .storage
                .all()
                .into_iter()
                .filter(|p| p.stock >= min_stock)
                .collect()),
        }
    }
}

/// CQRS query handler that returns acknowledgments
struct ProductQueryHandler {
    direct_handler: ProductDirectQueryHandler,
    // In real system, would publish results to event stream
}

impl ProductQueryHandler {
    fn new() -> Self {
        Self {
            direct_handler: ProductDirectQueryHandler::new(),
        }
    }
}

impl QueryHandler<ProductQuery> for ProductQueryHandler {
    fn handle(&self, envelope: QueryEnvelope<ProductQuery>) -> QueryResponse {
        // In a real system, this would:
        // 1. Process the query
        // 2. Publish results to an event stream
        // 3. Return acknowledgment with correlation ID

        // For demo, we'll just validate and return acknowledgment
        let result = self.direct_handler.handle(envelope.query.clone());

        match result {
            Ok(products) => {
                println!(
                    "   Query processed successfully, found {} products",
                    products.len()
                );
                QueryResponse {
                    query_id: IdType::Uuid(*envelope.id.as_uuid()),
                    correlation_id: envelope.correlation_id().clone(),
                    result: json!({
                        "status": "success",
                        "count": products.len(),
                        "products": products
                    }),
                }
            }
            Err(error) => {
                println!("   Query failed: {}", error);
                QueryResponse {
                    query_id: IdType::Uuid(*envelope.id.as_uuid()),
                    correlation_id: envelope.correlation_id().clone(),
                    result: json!({
                        "status": "error",
                        "error": error
                    }),
                }
            }
        }
    }
}

/// Example using QueryCriteria with read model
fn demonstrate_query_criteria(storage: &InMemoryReadModel<ProductView>) {
    println!("\n5. Using QueryCriteria...");

    // Create criteria for electronics under $100
    let mut criteria = QueryCriteria::new();
    criteria
        .filters
        .insert("category".to_string(), json!("Electronics"));
    criteria
        .filters
        .insert("max_price".to_string(), json!(100.0));
    criteria.limit = Some(10);

    // Custom query implementation
    let results: Vec<ProductView> = storage
        .all()
        .into_iter()
        .filter(|p| {
            criteria
                .filters
                .get("category")
                .and_then(|v| v.as_str())
                .map_or(true, |cat| p.category == cat)
                && criteria
                    .filters
                    .get("max_price")
                    .and_then(|v| v.as_f64())
                    .map_or(true, |max| p.price <= max)
        })
        .take(criteria.limit.unwrap_or(usize::MAX))
        .collect();

    println!("   Found {} products matching criteria", results.len());
    for product in results {
        println!("     - {} (${:.2})", product.name, product.price);
    }
}

fn main() {
    println!("Query Handler Example");
    println!("====================\n");

    // Create handlers
    let direct_handler = ProductDirectQueryHandler::new();
    let cqrs_handler = ProductQueryHandler::new();

    // Example 1: Direct query (returns data)
    println!("1. Direct query - Get by ID...");
    let query = ProductQuery::GetById {
        id: "prod-1".to_string(),
    };

    match direct_handler.handle(query) {
        Ok(products) => {
            if let Some(product) = products.first() {
                println!("   Found product: {}", product.name);
                println!("   Price: ${:.2}", product.price);
                println!("   Stock: {}", product.stock);
            } else {
                println!("   Product not found");
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 2: Search by name
    println!("\n2. Search by name...");
    let search_query = ProductQuery::SearchByName {
        name_contains: "desk".to_string(),
    };

    match direct_handler.handle(search_query) {
        Ok(products) => {
            println!("   Found {} products:", products.len());
            for product in products {
                println!("     - {} (${:.2})", product.name, product.price);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 3: CQRS query (returns acknowledgment)
    println!("\n3. CQRS query with acknowledgment...");
    let category_query = ProductQuery::FindByCategory {
        category: "Electronics".to_string(),
        min_price: Some(40.0),
        max_price: Some(1500.0),
    };

    let envelope = QueryEnvelope::new(category_query, "user-456".to_string());

    println!("   Query envelope:");
    println!("     ID: {}", envelope.id);
    println!("     Issued by: {}", envelope.issued_by);
    println!("     Correlation ID: {}", envelope.correlation_id());

    let response = cqrs_handler.handle(envelope);

    println!("   Response:");
    println!("     Query ID: {:?}", response.query_id);
    println!("     Result: {}", response.result);

    // Example 4: Complex query
    println!("\n4. Complex query - In stock products...");
    let stock_query = ProductQuery::GetInStock { min_stock: 10 };

    match direct_handler.handle(stock_query) {
        Ok(products) => {
            println!("   Products with 10+ stock:");
            for product in products {
                println!("     - {} (stock: {})", product.name, product.stock);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 5: Query criteria
    demonstrate_query_criteria(&direct_handler.storage);

    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • Direct query handlers that return data");
    println!("  • CQRS query handlers that return acknowledgments");
    println!("  • Different query patterns (by ID, search, filter)");
    println!("  • Query envelopes with metadata");
    println!("  • QueryCriteria for flexible filtering");

    // Show the two patterns
    println!("\nTwo Query Patterns:");
    println!("  1. DirectQueryHandler - Returns data directly (internal use)");
    println!("  2. QueryHandler - Returns acknowledgment (CQRS pattern)");
    println!("     - Results published to event stream");
    println!("     - Client subscribes using correlation ID");
}
