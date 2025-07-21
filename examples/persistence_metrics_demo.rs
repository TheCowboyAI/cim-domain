// Copyright 2025 Cowboy AI, LLC.

//! Demonstrates the persistence metrics collection capabilities
//!
//! This example shows how to:
//! - Wrap a repository with instrumentation
//! - Perform various operations
//! - Collect and display metrics
//! - Use metrics for performance monitoring

use cim_domain::{
    persistence::{InstrumentedRepository, NatsSimpleRepository, SimpleRepository},
    DomainEntity, EntityId,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Define a marker type
#[derive(Debug, Clone, Copy)]
struct ProductMarker;

// Define our domain entity
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: EntityId<ProductMarker>,
    name: String,
    price: f64,
    category: String,
    in_stock: bool,
}

impl Product {
    fn new(name: impl Into<String>, price: f64, category: impl Into<String>) -> Self {
        Self {
            id: EntityId::new(),
            name: name.into(),
            price,
            category: category.into(),
            in_stock: true,
        }
    }
}

impl DomainEntity for Product {
    type IdType = ProductMarker;

    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CIM Domain Persistence Metrics Demo ===\n");

    // Connect to NATS
    let client = async_nats::connect("nats://localhost:4222").await?;
    println!("✅ Connected to NATS");

    // Create base repository
    let base_repo = NatsSimpleRepository::new(
        client.clone(),
        "products_metrics_demo".to_string(),
        "Product".to_string(),
    )
    .await?;

    // Wrap with instrumentation
    let repo = InstrumentedRepository::new(base_repo);
    println!("✅ Created instrumented repository\n");

    // Perform various operations
    println!("📊 Performing operations...");

    // 1. Save products
    let products = vec![
        Product::new("Laptop", 999.99, "Electronics"),
        Product::new("Mouse", 29.99, "Electronics"),
        Product::new("Keyboard", 79.99, "Electronics"),
        Product::new("Monitor", 299.99, "Electronics"),
        Product::new("Desk", 199.99, "Furniture"),
        Product::new("Chair", 149.99, "Furniture"),
        Product::new("Lamp", 39.99, "Furniture"),
        Product::new("Notebook", 4.99, "Stationery"),
        Product::new("Pen", 1.99, "Stationery"),
        Product::new("Pencil", 0.99, "Stationery"),
    ];

    println!("  Saving {} products...", products.len());
    for product in &products {
        repo.save(product).await?;
    }

    // 2. Load existing products
    println!("  Loading products...");
    for product in &products[..5] {
        let _ = repo.load(&product.id).await?;
    }

    // 3. Check existence
    println!("  Checking existence...");
    for product in &products[..3] {
        let _ = repo.exists(&product.id).await?;
    }

    // 4. Load non-existent products (misses)
    println!("  Loading non-existent products...");
    for _ in 0..5 {
        let fake_id = EntityId::<ProductMarker>::new();
        let _ = repo.load(&fake_id).await?;
    }

    // 5. Concurrent operations
    println!("  Performing concurrent operations...");
    let mut handles = vec![];

    // Clone repo for concurrent access
    for i in 0..10 {
        let repo = repo.clone();
        let handle = tokio::spawn(async move {
            let product = Product::new(
                format!("Concurrent Product {}", i),
                99.99 + i as f64,
                "Test",
            );
            repo.save(&product).await
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;
    for result in results {
        result??;
    }

    // 6. Simulate some errors
    println!("  Simulating errors...");
    // This would normally cause errors - for demo we'll skip actual errors

    println!("\n✅ Operations completed\n");

    // Display metrics summary
    display_metrics_summary(&repo).await;

    // Display detailed metrics
    display_detailed_metrics(&repo).await;

    // Display performance analysis
    display_performance_analysis(&repo).await;

    Ok(())
}

async fn display_metrics_summary<R: SimpleRepository<Product>>(
    repo: &InstrumentedRepository<Product, R>,
) {
    println!("📊 === Metrics Summary ===\n");

    let metrics = repo.metrics();
    let summary = metrics.summary().await;

    println!("📈 Operation Counts:");
    for (counter, count) in &summary.counters {
        println!("  {}: {}", counter, count);
    }

    println!("\n❌ Errors:");
    if summary.errors.is_empty() {
        println!("  No errors recorded! 🎉");
    } else {
        for (operation, count) in &summary.errors {
            println!("  {}: {} errors", operation, count);
        }
    }

    println!("\n⏱️  Performance Summary:");
    for (operation, stats) in &summary.durations {
        println!("  {}:", operation);
        println!("    Count: {}", stats.count);
        println!("    Average: {:?}", stats.avg);
        println!("    P50: {:?}", stats.p50);
        println!("    P95: {:?}", stats.p95);
        println!("    P99: {:?}", stats.p99);
        println!("    Min: {:?}", stats.min);
        println!("    Max: {:?}", stats.max);
    }
}

async fn display_detailed_metrics<R: SimpleRepository<Product>>(
    repo: &InstrumentedRepository<Product, R>,
) {
    println!("\n📊 === Detailed Metrics ===\n");

    let metrics = repo.metrics();

    // Repository operations
    let save_count = metrics.get_counter("repository.save.count").await;
    let save_success = metrics.get_counter("repository.save.success").await;
    let save_errors = metrics.get_counter("repository.save.error").await;

    println!("💾 Save Operations:");
    println!("  Total: {}", save_count);
    println!(
        "  Success: {} ({:.1}%)",
        save_success,
        (save_success as f64 / save_count as f64) * 100.0
    );
    println!(
        "  Errors: {} ({:.1}%)",
        save_errors,
        (save_errors as f64 / save_count as f64) * 100.0
    );

    let load_count = metrics.get_counter("repository.load.count").await;
    let load_hits = metrics.get_counter("repository.load.hit").await;
    let load_misses = metrics.get_counter("repository.load.miss").await;
    let load_errors = metrics.get_counter("repository.load.error").await;

    println!("\n📖 Load Operations:");
    println!("  Total: {}", load_count);
    println!(
        "  Hits: {} ({:.1}%)",
        load_hits,
        (load_hits as f64 / load_count as f64) * 100.0
    );
    println!(
        "  Misses: {} ({:.1}%)",
        load_misses,
        (load_misses as f64 / load_count as f64) * 100.0
    );
    println!(
        "  Errors: {} ({:.1}%)",
        load_errors,
        (load_errors as f64 / load_count as f64) * 100.0
    );

    let exists_count = metrics.get_counter("repository.exists.count").await;
    let exists_true = metrics.get_counter("repository.exists.true").await;
    let exists_false = metrics.get_counter("repository.exists.false").await;

    println!("\n🔍 Exists Operations:");
    println!("  Total: {}", exists_count);
    println!(
        "  Found: {} ({:.1}%)",
        exists_true,
        (exists_true as f64 / exists_count as f64) * 100.0
    );
    println!(
        "  Not Found: {} ({:.1}%)",
        exists_false,
        (exists_false as f64 / exists_count as f64) * 100.0
    );
}

async fn display_performance_analysis<R: SimpleRepository<Product>>(
    repo: &InstrumentedRepository<Product, R>,
) {
    println!("\n📊 === Performance Analysis ===\n");

    let metrics = repo.metrics();

    // Compare operation performance
    println!("⚡ Operation Performance Comparison:");

    if let Some(save_avg) = metrics.get_avg_duration("repository.save").await {
        println!("  Save avg: {:?}", save_avg);
    }

    if let Some(load_avg) = metrics.get_avg_duration("repository.load").await {
        println!("  Load avg: {:?}", load_avg);
    }

    if let Some(exists_avg) = metrics.get_avg_duration("repository.exists").await {
        println!("  Exists avg: {:?}", exists_avg);
    }

    // Performance recommendations
    println!("\n💡 Performance Insights:");

    let summary = metrics.summary().await;

    // Check for slow operations
    for (op, stats) in &summary.durations {
        if stats.p99 > Duration::from_millis(100) {
            println!("  ⚠️  {} has high P99 latency: {:?}", op, stats.p99);
            println!("     Consider optimizing or adding caching");
        }

        if stats.max > Duration::from_millis(500) {
            println!("  ⚠️  {} has very high max latency: {:?}", op, stats.max);
            println!("     Investigate potential timeout issues");
        }
    }

    // Check cache effectiveness
    let load_hits = metrics.get_counter("repository.load.hit").await;
    let load_misses = metrics.get_counter("repository.load.miss").await;
    if load_hits + load_misses > 0 {
        let hit_rate = load_hits as f64 / (load_hits + load_misses) as f64;
        if hit_rate < 0.8 {
            println!("  ⚠️  Low cache hit rate: {:.1}%", hit_rate * 100.0);
            println!("     Consider implementing a caching layer");
        } else {
            println!("  ✅ Good cache hit rate: {:.1}%", hit_rate * 100.0);
        }
    }

    // Check error rates
    for (op, &error_count) in &summary.errors {
        if let Some(&total) = summary.counters.get(&format!("{}.count", op)) {
            let error_rate = error_count as f64 / total as f64;
            if error_rate > 0.01 {
                println!(
                    "  ⚠️  High error rate for {}: {:.1}%",
                    op,
                    error_rate * 100.0
                );
                println!("     Investigate error causes");
            }
        }
    }

    println!("\n✅ Metrics collection complete!");
}
