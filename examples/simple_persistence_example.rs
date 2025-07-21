// Copyright 2025 Cowboy AI, LLC.

//! Simple example demonstrating basic persistence with NATS JetStream
//!
//! This example shows how to:
//! - Use the simple repository for basic persistence
//! - Store and retrieve aggregates using NATS KV

use cim_domain::{
    persistence::{NatsSimpleRepository, SimpleRepository},
    DomainEntity, EntityId,
};
use serde::{Deserialize, Serialize};

/// Example aggregate: Product
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: EntityId<Product>,
    name: String,
    price: f64,
    stock: u32,
}

impl DomainEntity for Product {
    type IdType = Product;

    fn id(&self) -> EntityId<Self::IdType> {
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
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple Persistence Example");
    println!("=========================\n");

    // Note: This example requires a running NATS server with JetStream enabled
    // Run: docker run -p 4222:4222 nats:latest -js

    // Connect to NATS
    println!("1. Connecting to NATS...");
    let client = async_nats::connect("nats://localhost:4222").await?;
    println!("   ✓ Connected\n");

    // Create repository
    println!("2. Creating repository...");
    let repository: Box<dyn SimpleRepository<Product>> = Box::new(
        NatsSimpleRepository::new(client, "products".to_string(), "Product".to_string()).await?,
    );
    println!("   ✓ Repository ready\n");

    // Create a product
    println!("3. Creating product...");
    let product = Product::new("Laptop Pro X1".to_string(), 1299.99, 50);
    let product_id = product.id().clone();
    println!("   Product ID: {}", product_id);
    println!("   Name: {}", product.name);
    println!("   Price: ${}", product.price);
    println!("   Stock: {}\n", product.stock);

    // Save the product
    println!("4. Saving product...");
    let metadata = repository.save(&product).await?;
    println!("   ✓ Saved successfully");
    println!("   Version: {}", metadata.version);
    println!("   Subject: {}\n", metadata.subject);

    // Check if exists
    println!("5. Checking existence...");
    let exists = repository.exists(&product_id).await?;
    println!("   Exists: {}\n", exists);

    // Load the product
    println!("6. Loading product...");
    let loaded_result: Option<Product> = repository.load(&product_id).await?;
    match loaded_result {
        Some(loaded) => {
            println!("   ✓ Product loaded");
            println!("   Name: {}", loaded.name);
            println!("   Price: ${}", loaded.price);
            println!("   Stock: {}", loaded.stock);
        }
        None => {
            println!("   ✗ Product not found");
        }
    }

    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • Basic NATS JetStream persistence");
    println!("  • Simple key-value storage");
    println!("  • Subject-based addressing");
    println!("  • Aggregate serialization");

    Ok(())
}
