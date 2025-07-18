// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating the persistence layer with NATS JetStream
//!
//! This example shows how to:
//! - Set up NATS-based persistence
//! - Store and retrieve aggregates
//! - Use read models for queries

use cim_domain::{
    EntityId,
    DomainEntity,
    persistence::*,
    DomainError,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Example aggregate: Product
#[derive(Debug, Clone, Copy)]
struct ProductMarker;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: EntityId<ProductMarker>,
    name: String,
    price: f64,
    stock: u32,
    version: u64,
}

impl DomainEntity for Product {
    type IdType = ProductMarker;
    
    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

impl Product {
    fn new(name: String, price: f64, stock: u32) -> Self {
        Self {
            id: EntityId::new(),
            name,
            price,
            stock,
            version: 1,
        }
    }
    
    fn update_price(&mut self, new_price: f64) {
        self.price = new_price;
        self.version += 1;
    }
    
    fn update_stock(&mut self, new_stock: u32) {
        self.stock = new_stock;
        self.version += 1;
    }
}

/// Product statistics read model
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductStats {
    id: String,
    total_products: u32,
    total_value: f64,
    out_of_stock_count: u32,
    last_updated: chrono::DateTime<chrono::Utc>,
}

impl ReadModel for ProductStats {
    fn model_type() -> &'static str {
        "ProductStats"
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn apply_event(&mut self, _event: &dyn cim_domain::DomainEvent) -> Result<(), DomainError> {
        // In a real implementation, we'd update stats based on events
        self.last_updated = Utc::now();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 CIM Domain - Persistence Example");
    println!("==================================\n");
    
    // Connect to NATS
    println!("📡 Connecting to NATS...");
    let client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => {
            println!("✅ Connected to NATS\n");
            client
        }
        Err(e) => {
            println!("❌ Failed to connect to NATS: {}", e);
            println!("\nPlease ensure NATS is running:");
            println!("  docker run -p 4222:4222 nats:latest -js");
            return Ok(());
        }
    };
    
    // Part 1: Simple Repository
    println!("📦 Part 1: Simple Repository");
    println!("---------------------------");
    
    let simple_repo = NatsSimpleRepository::new(
        client.clone(),
        "products-simple".to_string(),
        "Product".to_string(),
    ).await?;
    
    // Create and save a product
    let mut product = Product::new("Laptop".to_string(), 999.99, 10);
    let product_id = product.id();
    
    println!("  Creating product: {} (${:.2})", product.name, product.price);
    let metadata = simple_repo.save(&product).await?;
    println!("  ✅ Saved with version: {}", metadata.version);
    
    // Update and save
    product.update_price(899.99);
    println!("  Updating price to: ${:.2}", product.price);
    let metadata = simple_repo.save(&product).await?;
    println!("  ✅ Updated to version: {}", metadata.version);
    
    // Load the product
    let loaded: Option<Product> = simple_repo.load(&product_id).await?;
    if let Some(loaded_product) = loaded {
        println!("  ✅ Loaded product: {} (${:.2})", loaded_product.name, loaded_product.price);
    }
    
    // Part 2: NATS KV Repository with Builder
    println!("\n📦 Part 2: NATS KV Repository");
    println!("-----------------------------");
    
    let kv_repo: NatsKvRepository<Product> = NatsKvRepositoryBuilder::new()
        .client(client.clone())
        .bucket_name("products-kv")
        .aggregate_type("Product")
        .history(20)
        .ttl_seconds(3600) // 1 hour TTL
        .build()
        .await?;
    
    // Create products
    let products = vec![
        Product::new("Mouse".to_string(), 29.99, 50),
        Product::new("Keyboard".to_string(), 79.99, 30),
        Product::new("Monitor".to_string(), 299.99, 5),
        Product::new("Webcam".to_string(), 59.99, 0), // Out of stock
    ];
    
    for product in &products {
        let metadata = kv_repo.save(product).await?;
        println!("  Saved {} (stock: {}) - version: {}", 
            product.name, product.stock, metadata.version);
    }
    
    // Part 3: Read Model Store
    println!("\n📊 Part 3: Read Model Store");
    println!("---------------------------");
    
    let read_model_store = NatsReadModelStore::new(
        client.clone(),
        "product-read-models".to_string(),
    ).await?;
    
    // Create and save product statistics
    let stats = ProductStats {
        id: "global-stats".to_string(),
        total_products: products.len() as u32,
        total_value: products.iter().map(|p| p.price * p.stock as f64).sum(),
        out_of_stock_count: products.iter().filter(|p| p.stock == 0).count() as u32,
        last_updated: Utc::now(),
    };
    
    let stats_metadata = ReadModelMetadata {
        id: stats.id.clone(),
        model_type: ProductStats::model_type().to_string(),
        schema_version: 1,
        last_updated: Utc::now(),
        last_event_position: 100,
        metadata: HashMap::new(),
    };
    
    read_model_store.save(&stats, stats_metadata).await?;
    println!("  Saved product statistics");
    println!("  Total products: {}", stats.total_products);
    println!("  Total inventory value: ${:.2}", stats.total_value);
    println!("  Out of stock items: {}", stats.out_of_stock_count);
    
    // Update projection status
    read_model_store.update_projection_status(
        ProductStats::model_type(),
        ProjectionStatus::UpToDate,
    ).await?;
    println!("  ✅ Projection status: Up to date");
    
    // Part 4: Query Support
    println!("\n🔍 Part 4: Query Support");
    println!("------------------------");
    
    // Build a query
    let query = QueryBuilder::new()
        .filter("category", serde_json::json!("electronics"))
        .filter("in_stock", serde_json::json!(true))
        .sort_by("price", SortDirection::Ascending)
        .limit(10)
        .offset(0)
        .build();
    
    println!("  Built query with:");
    println!("    Filters: {} filters", query.filters.len());
    println!("    Sort: {:?}", query.sort_by);
    println!("    Limit: {:?}", query.limit);
    
    // Demonstrate pagination
    let total_items = 50;
    let pagination = Pagination::from_query(10, 20, total_items);
    
    println!("\n  Pagination info:");
    println!("    Current page: {}/{}", pagination.page, pagination.total_pages);
    println!("    Items per page: {}", pagination.per_page);
    println!("    Has next page: {}", pagination.has_next());
    println!("    Has previous page: {}", pagination.has_prev());
    
    // Part 5: Advanced Features
    println!("\n🚀 Part 5: Advanced Features");
    println!("----------------------------");
    
    // Demonstrate save options
    let save_options = SaveOptions {
        expected_version: Some(2),
        create_snapshot: true,
        metadata: None,
    };
    
    println!("  Save options:");
    println!("    Expected version: {:?}", save_options.expected_version);
    println!("    Create snapshot: {}", save_options.create_snapshot);
    
    // Demonstrate load options
    let load_options = LoadOptions {
        version: Some(1),
        use_snapshot: true,
        max_events: Some(100),
    };
    
    println!("\n  Load options:");
    println!("    Specific version: {:?}", load_options.version);
    println!("    Use snapshot: {}", load_options.use_snapshot);
    println!("    Max events: {:?}", load_options.max_events);
    
    println!("\n✅ Persistence example completed successfully!");
    
    Ok(())
}