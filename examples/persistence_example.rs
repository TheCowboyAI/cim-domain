// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating the persistence layer with NATS JetStream
//!
//! This example shows how to:
//! - Set up NATS-based persistence
//! - Store and retrieve aggregates
//! - Use read models for queries
//! - Optimize queries with subject-based routing
//! - Handle schema migrations

use cim_domain::{
    entity::{Entity, EntityId},
    events::DomainEvent,
    persistence::*,
    infrastructure::{JetStreamEventStore, JetStreamConfig},
    DomainError,
};
use async_nats::Client;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Example aggregate: Product
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: EntityId<Product>,
    name: String,
    price: f64,
    stock: u32,
    version: u64,
}

impl Entity for Product {
    type IdType = Product;
    
    fn id(&self) -> EntityId<Self> {
        self.id.clone()
    }
}

impl Product {
    fn new(name: String, price: f64, stock: u32) -> Self {
        Self {
            id: EntityId::new(),
            name,
            price,
            stock,
            version: 0,
        }
    }
    
    fn update_price(&mut self, new_price: f64) -> ProductPriceUpdated {
        let event = ProductPriceUpdated {
            product_id: self.id.value(),
            old_price: self.price,
            new_price,
            updated_at: Utc::now(),
        };
        
        self.price = new_price;
        self.version += 1;
        
        event
    }
    
    fn update_stock(&mut self, quantity: i32) -> Result<StockUpdated, DomainError> {
        let new_stock = (self.stock as i32 + quantity) as u32;
        
        if (self.stock as i32 + quantity) < 0 {
            return Err(DomainError::ValidationError("Insufficient stock".to_string()));
        }
        
        let event = StockUpdated {
            product_id: self.id.value(),
            old_stock: self.stock,
            new_stock,
            change: quantity,
            updated_at: Utc::now(),
        };
        
        self.stock = new_stock;
        self.version += 1;
        
        Ok(event)
    }
}

/// Product events
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductCreated {
    product_id: Uuid,
    name: String,
    price: f64,
    stock: u32,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductPriceUpdated {
    product_id: Uuid,
    old_price: f64,
    new_price: f64,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StockUpdated {
    product_id: Uuid,
    old_stock: u32,
    new_stock: u32,
    change: i32,
    updated_at: chrono::DateTime<Utc>,
}

// Implement DomainEvent for events
impl DomainEvent for ProductCreated {
    fn event_type(&self) -> &'static str {
        "ProductCreated"
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.product_id
    }
    
    fn subject(&self) -> String {
        "domain.product.created.v1".to_string()
    }
}

impl DomainEvent for ProductPriceUpdated {
    fn event_type(&self) -> &'static str {
        "ProductPriceUpdated"
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.product_id
    }
    
    fn subject(&self) -> String {
        "domain.product.price_updated.v1".to_string()
    }
}

impl DomainEvent for StockUpdated {
    fn event_type(&self) -> &'static str {
        "StockUpdated"
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.product_id
    }
    
    fn subject(&self) -> String {
        "domain.product.stock_updated.v1".to_string()
    }
}

/// Read model for product catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductCatalogView {
    id: String,
    products: HashMap<String, ProductSummary>,
    total_products: u32,
    last_updated: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductSummary {
    id: String,
    name: String,
    price: f64,
    in_stock: bool,
}

impl ReadModel for ProductCatalogView {
    fn model_type() -> &'static str {
        "ProductCatalog"
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn apply_event(&mut self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        match event.event_type() {
            "ProductCreated" => {
                // In real implementation, would deserialize event data
                let product_id = event.aggregate_id().to_string();
                self.products.insert(
                    product_id.clone(),
                    ProductSummary {
                        id: product_id,
                        name: "New Product".to_string(),
                        price: 0.0,
                        in_stock: true,
                    }
                );
                self.total_products += 1;
            }
            "ProductPriceUpdated" => {
                let product_id = event.aggregate_id().to_string();
                if let Some(product) = self.products.get_mut(&product_id) {
                    // Update price from event data
                    product.price = 99.99; // Placeholder
                }
            }
            "StockUpdated" => {
                let product_id = event.aggregate_id().to_string();
                if let Some(product) = self.products.get_mut(&product_id) {
                    // Update stock status from event data
                    product.in_stock = true; // Placeholder
                }
            }
            _ => {}
        }
        
        self.last_updated = Utc::now();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("CIM Domain Persistence Example");
    println!("==============================\n");
    
    // Note: This example requires a running NATS server with JetStream enabled
    // Run: docker run -p 4222:4222 nats:latest -js
    
    // Connect to NATS
    println!("1. Connecting to NATS...");
    let client = Client::connect("nats://localhost:4222").await?;
    println!("   ✓ Connected to NATS\n");
    
    // Set up persistence
    println!("2. Setting up persistence layer...");
    
    // Configure NATS repository
    let config = NatsRepositoryConfig {
        event_stream_name: "PRODUCTS-EVENTS".to_string(),
        aggregate_stream_name: "PRODUCTS-AGGREGATES".to_string(),
        read_model_bucket: "products-read-models".to_string(),
        snapshot_bucket: "products-snapshots".to_string(),
        event_subject_prefix: "products.events".to_string(),
        aggregate_subject_prefix: "products.aggregates".to_string(),
        cache_size: 100,
        enable_ipld: true,
    };
    
    let repository: NatsRepository<Product> = NatsRepository::new(
        client.clone(),
        config.clone(),
        "Product".to_string(),
    ).await?;
    
    println!("   ✓ Repository configured\n");
    
    // Create read model store
    let read_store = NatsReadModelStore::new(client.clone(), "products").await?;
    println!("   ✓ Read model store created\n");
    
    // Set up query optimizer
    let optimizer = NatsQueryOptimizer::new();
    
    // Create indexes
    optimizer.create_index(
        "product_subject_index".to_string(),
        vec![
            "products.aggregates.Product.*".to_string(),
            "products.events.Product.*".to_string(),
        ],
    ).await?;
    
    println!("   ✓ Query optimizer configured\n");
    
    // 3. Create and save a product
    println!("3. Creating and saving a product...");
    
    let mut product = Product::new(
        "Laptop Pro X1".to_string(),
        1299.99,
        50,
    );
    
    // Generate events
    let created_event = ProductCreated {
        product_id: product.id.value(),
        name: product.name.clone(),
        price: product.price,
        stock: product.stock,
        created_at: Utc::now(),
    };
    
    // Save with events
    let metadata = repository.save(
        &product,
        vec![Box::new(created_event)],
        SaveOptions {
            expected_version: None,
            create_snapshot: true,
            metadata: HashMap::from([
                ("category".to_string(), serde_json::json!("electronics")),
            ]),
            tags: vec!["new".to_string(), "featured".to_string()],
        },
    ).await?;
    
    println!("   ✓ Product saved: {}", metadata.aggregate_id);
    println!("   ✓ Version: {}", metadata.version);
    println!("   ✓ Subject: {}", metadata.subject);
    println!("   ✓ CID: {}\n", metadata.state_cid);
    
    // 4. Update the product
    println!("4. Updating product price...");
    
    let price_event = product.update_price(1199.99);
    
    let metadata = repository.save(
        &product,
        vec![Box::new(price_event)],
        SaveOptions {
            expected_version: Some(1),
            create_snapshot: false,
            metadata: HashMap::new(),
            tags: vec![],
        },
    ).await?;
    
    println!("   ✓ Price updated to ${}", product.price);
    println!("   ✓ New version: {}\n", metadata.version);
    
    // 5. Load the product
    println!("5. Loading product from persistence...");
    
    let (loaded_product, metadata) = repository.load(
        &product.id,
        LoadOptions {
            version: None,
            as_of: None,
            include_events: true,
            use_snapshot: true,
        },
    ).await?;
    
    println!("   ✓ Loaded product: {}", loaded_product.name);
    println!("   ✓ Price: ${}", loaded_product.price);
    println!("   ✓ Stock: {}", loaded_product.stock);
    println!("   ✓ Version: {}\n", metadata.version);
    
    // 6. Create and update read model
    println!("6. Creating product catalog view...");
    
    let mut catalog = ProductCatalogView {
        id: "main-catalog".to_string(),
        products: HashMap::new(),
        total_products: 0,
        last_updated: Utc::now(),
    };
    
    // Apply events to build view
    catalog.apply_event(&ProductCreated {
        product_id: product.id.value(),
        name: product.name.clone(),
        price: product.price,
        stock: product.stock,
        created_at: Utc::now(),
    })?;
    
    // Save read model
    read_store.save(
        &catalog,
        ReadModelMetadata {
            id: catalog.id.clone(),
            model_type: ProductCatalogView::model_type().to_string(),
            schema_version: 1,
            last_updated: Utc::now(),
            last_event_position: 2,
            data_cid: None,
            subject: "readmodel.product.catalog".to_string(),
            metadata: HashMap::new(),
        },
    ).await?;
    
    println!("   ✓ Catalog view created with {} products\n", catalog.total_products);
    
    // 7. Query optimization
    println!("7. Demonstrating query optimization...");
    
    let query = "subject:products.aggregates.Product.*";
    let hints = QueryHint {
        strategy: Some(IndexStrategy::SubjectPattern),
        expected_size: Some(10),
        time_range: None,
        projection: Some(vec!["id".to_string(), "name".to_string(), "price".to_string()]),
        use_cache: true,
    };
    
    let plan = optimizer.create_plan(query, hints).await?;
    
    println!("   ✓ Query plan created");
    println!("   → Strategy: {:?}", plan.strategy);
    println!("   → Estimated cost: {}", plan.estimated_cost);
    println!("   → Steps: {}", plan.steps.len());
    
    for step in &plan.steps {
        println!("     - {}: {:?}", step.name, step.operation);
    }
    
    println!();
    
    // 8. Subject routing
    println!("8. Setting up subject-based routing...");
    
    let router = SubjectRouter::new(RoutingStrategy::PriorityBased);
    
    // Add route for product events
    router.add_route(RoutePattern {
        name: "product_events".to_string(),
        pattern: "products.events.*".to_string(),
        handler: "event_processor".to_string(),
        priority: 10,
        metadata: HashMap::new(),
    }).await?;
    
    // Add route for product aggregates
    router.add_route(RoutePattern {
        name: "product_aggregates".to_string(),
        pattern: "products.aggregates.*".to_string(),
        handler: "aggregate_processor".to_string(),
        priority: 5,
        metadata: HashMap::new(),
    }).await?;
    
    let stats = router.get_stats().await;
    println!("   ✓ Router configured");
    println!("   → Total routes: {}", stats.total_routes);
    println!("   → Strategy: {:?}\n", stats.strategy);
    
    // 9. Schema migration
    println!("9. Demonstrating schema migration...");
    
    let current_version = SchemaVersion::new(1, 0, 0);
    let target_version = SchemaVersion::new(1, 1, 0);
    
    let mut migration_runner = MigrationRunner::new(current_version);
    
    println!("   → Current version: {}", migration_runner.current_version());
    println!("   → Target version: {}", target_version);
    println!("   → Migration path would be calculated here\n");
    
    // 10. Content-addressed storage with IPLD
    println!("10. Using IPLD for content addressing...");
    
    let mut serializer = IpldSerializer::new();
    
    // Add product to content chain
    let product_data = serde_json::to_vec(&product)?;
    let cid = serializer.add_to_chain(
        "product-history",
        product_data,
        HashMap::from([
            ("version".to_string(), "2".to_string()),
            ("type".to_string(), "product".to_string()),
        ]),
    )?;
    
    println!("   ✓ Product added to IPLD chain");
    println!("   → CID: {}", cid);
    println!("   → Chain verified: {}\n", serializer.verify_chain("product-history")?);
    
    println!("✅ Persistence example completed successfully!");
    println!("\nThe persistence layer provides:");
    println!("  • NATS JetStream for event and aggregate storage");
    println!("  • NATS KV for read model persistence");
    println!("  • Subject-based routing and indexing");
    println!("  • Query optimization with patterns");
    println!("  • Content-addressed storage with IPLD");
    println!("  • Schema migration support");
    
    Ok(())
}