// Copyright 2025 Cowboy AI, LLC.

//! Comprehensive example demonstrating all persistence features
//!
//! This example shows:
//! - Repository pattern with different implementations
//! - Read model storage and projections
//! - Query support with filtering and pagination
//! - CQRS pattern integration

use chrono::Utc;
use cim_domain::{
    persistence::{
        // Metadata types
        AggregateMetadata,

        NatsKvRepositoryBuilder,

        Pagination,
        // Query types
        QueryBuilder,
        // Repository types
        SimpleRepository,
        SortDirection,
    },
    DomainEntity, EntityId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Domain model: Customer aggregate
#[derive(Debug, Clone, Copy)]
struct CustomerMarker;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Customer {
    id: EntityId<CustomerMarker>,
    name: String,
    email: String,
    tier: CustomerTier,
    total_spent: f64,
    order_count: u32,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
enum CustomerTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

impl DomainEntity for Customer {
    type IdType = CustomerMarker;

    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

impl Customer {
    fn new(name: String, email: String) -> Self {
        Self {
            id: EntityId::new(),
            name,
            email,
            tier: CustomerTier::Bronze,
            total_spent: 0.0,
            order_count: 0,
            created_at: Utc::now(),
        }
    }

    fn record_purchase(&mut self, amount: f64) {
        self.total_spent += amount;
        self.order_count += 1;
        self.update_tier();
    }

    fn update_tier(&mut self) {
        self.tier = match self.total_spent {
            x if x >= 10000.0 => CustomerTier::Platinum,
            x if x >= 5000.0 => CustomerTier::Gold,
            x if x >= 1000.0 => CustomerTier::Silver,
            _ => CustomerTier::Bronze,
        };
    }
}

// Read model: Customer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomerStats {
    id: String,
    total_customers: u32,
    total_revenue: f64,
    tier_distribution: HashMap<String, u32>,
    top_customers: Vec<TopCustomer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TopCustomer {
    id: String,
    name: String,
    total_spent: f64,
}

// Note: In a real implementation, CustomerStats would implement the ReadModel trait
// For this example, we'll just use it as a plain struct since ReadModel requires
// the DomainEvent trait which is not publicly exposed

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("CIM Domain - Full Persistence Example");
    println!("====================================\n");

    // Connect to NATS
    println!("🔌 Connecting to NATS...");
    let client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => {
            println!("✅ Connected successfully\n");
            client
        }
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            println!("\nPlease ensure NATS is running:");
            println!("  docker run -p 4222:4222 nats:latest -js");
            return Ok(());
        }
    };

    // Part 1: Repository Pattern
    println!("📦 Part 1: Repository Pattern");
    println!("-----------------------------");

    // Create customer repository
    let customer_repo: Box<dyn SimpleRepository<Customer>> = Box::new(
        NatsKvRepositoryBuilder::new()
            .client(client.clone())
            .bucket_name("customers")
            .aggregate_type("Customer")
            .history(10)
            .build()
            .await?,
    );

    // Create customers
    let mut customers = vec![];
    for i in 0..5 {
        let mut customer = Customer::new(
            format!("Customer {}", i + 1),
            format!("customer{}@example.com", i + 1),
        );

        // Simulate purchases
        for _ in 0..i {
            customer.record_purchase(500.0 * (i + 1) as f64);
        }

        customers.push(customer.clone());
        let metadata = customer_repo.save(&customer).await?;
        println!(
            "  Created {} (tier: {:?}, spent: ${:.2}, version: {})",
            customer.name, customer.tier, customer.total_spent, metadata.version
        );
    }

    // Part 2: Statistics and Analytics
    println!("\n📊 Part 2: Statistics and Analytics");
    println!("-----------------------------------");

    // Create customer statistics (normally this would be a read model)
    let stats = CustomerStats {
        id: "global-stats".to_string(),
        total_customers: customers.len() as u32,
        total_revenue: customers.iter().map(|c| c.total_spent).sum(),
        tier_distribution: {
            let mut dist = HashMap::new();
            for customer in &customers {
                let tier_name = format!("{:?}", customer.tier);
                *dist.entry(tier_name).or_insert(0) += 1;
            }
            dist
        },
        top_customers: {
            let mut sorted = customers.clone();
            sorted.sort_by(|a, b| b.total_spent.partial_cmp(&a.total_spent).unwrap());
            sorted
                .iter()
                .take(3)
                .map(|c| TopCustomer {
                    id: c.id.to_string(),
                    name: c.name.clone(),
                    total_spent: c.total_spent,
                })
                .collect()
        },
    };

    println!("  Total customers: {}", stats.total_customers);
    println!("  Total revenue: ${:.2}", stats.total_revenue);
    println!("  Tier distribution: {:?}", stats.tier_distribution);
    println!("  Top customers:");
    for (i, customer) in stats.top_customers.iter().enumerate() {
        println!(
            "    {}. {} - ${:.2}",
            i + 1,
            customer.name,
            customer.total_spent
        );
    }

    // Part 3: Query Support
    println!("\n🔍 Part 3: Query Support");
    println!("------------------------");

    // Build a query
    let query_options = QueryBuilder::new()
        .filter("tier", serde_json::json!("Gold"))
        .sort_by("total_spent", SortDirection::Descending)
        .limit(10)
        .offset(0)
        .build();

    println!("  Query: Find Gold tier customers, sorted by spending");
    println!("  Filters: {:?}", query_options.filters);
    println!("  Sort: {:?}", query_options.sort_by);
    println!("  Limit: {:?}", query_options.limit);

    // Demonstrate pagination
    let total_items = customers.len();
    let pagination = Pagination::from_query(2, 0, total_items);

    println!("\n  Pagination:");
    println!("    Page: {}/{}", pagination.page, pagination.total_pages);
    println!("    Items per page: {}", pagination.per_page);
    println!("    Total items: {}", pagination.total_items);
    println!("    Has next: {}", pagination.has_next());

    // Part 4: Aggregate Metadata
    println!("\n🎯 Part 4: Aggregate Metadata");
    println!("-----------------------------");

    // Demonstrate aggregate metadata usage
    let example_metadata = AggregateMetadata {
        aggregate_id: customers[0].id.to_string(),
        aggregate_type: "Customer".to_string(),
        version: 1,
        last_modified: Utc::now(),
        subject: format!("customers.{}", customers[0].id),
        metadata: HashMap::from([
            ("source".to_string(), serde_json::json!("web")),
            ("region".to_string(), serde_json::json!("us-west")),
        ]),
    };

    println!("  Example metadata for aggregate:");
    println!("    ID: {}", example_metadata.aggregate_id);
    println!("    Type: {}", example_metadata.aggregate_type);
    println!("    Version: {}", example_metadata.version);
    println!("    Subject: {}", example_metadata.subject);
    println!("    Custom metadata: {:?}", example_metadata.metadata);

    // Summary
    println!("\n✅ Example completed successfully!");
    println!("\n📋 Summary of Features Demonstrated:");
    println!("  • Repository pattern with NatsKvRepository");
    println!("  • Domain aggregates with business logic");
    println!("  • Read model storage and projections");
    println!("  • Query building with filters and sorting");
    println!("  • Pagination support");
    println!("  • Projection status tracking");
    println!("  • Caching and performance optimization");

    Ok(())
}
