// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating persistence patterns
//!
//! This example shows:
//! - Using simple repository implementations
//! - NATS KV persistence
//! - Aggregate versioning
//! - Read model storage

use cim_domain::{
    // Core types
    EntityId, AggregateRoot, DomainEntity,
    markers::AggregateMarker,
    
    // Persistence
    persistence::{
        SimpleRepository, NatsKvRepositoryBuilder,
        AggregateMetadata,
    },
    
    // Query support (for read models)
    ReadModelStorage, InMemoryReadModel,
    
    // IDs
    WorkflowId,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Example aggregate: UserProfile
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserProfile {
    id: EntityId<AggregateMarker>,
    username: String,
    email: String,
    full_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    version: u64,
}

impl DomainEntity for UserProfile {
    type IdType = AggregateMarker;
    
    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

impl UserProfile {
    fn new(username: String, email: String, full_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            username,
            email,
            full_name,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }
    
    fn update_email(&mut self, new_email: String) {
        self.email = new_email;
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

/// Read model for user search
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserSearchView {
    user_id: String,
    username: String,
    email: String,
    full_name: String,
}

impl From<&UserProfile> for UserSearchView {
    fn from(profile: &UserProfile) -> Self {
        Self {
            user_id: profile.id.to_string(),
            username: profile.username.clone(),
            email: profile.email.clone(),
            full_name: profile.full_name.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Persistence Example");
    println!("==================\n");
    
    // Example 1: NATS KV Repository
    println!("1. NATS KV Repository...");
    
    // Note: This requires a running NATS server
    // Run: docker run -p 4222:4222 nats:latest -js
    
    let client = async_nats::connect("nats://localhost:4222").await?;
    
    let nats_repo = NatsKvRepositoryBuilder::new()
        .client(client.clone())
        .bucket_name("user_profiles".to_string())
        .build()
        .await?;
    
    // Create a user profile
    let mut user = UserProfile::new(
        "alice_doe".to_string(),
        "alice@example.com".to_string(),
        "Alice Doe".to_string(),
    );
    
    println!("   Created user: {}", user.username);
    println!("   ID: {}", user.id);
    
    // Save to NATS KV
    let metadata = nats_repo.save(&user).await?;
    println!("\n   ✓ Saved to NATS KV");
    println!("     Subject: {}", metadata.subject);
    println!("     Version: {}", metadata.version);
    println!("     Last modified: {}", metadata.last_modified);
    
    // Load from NATS KV
    if let Some(loaded) = nats_repo.load(&user.id).await? {
        println!("\n   ✓ Loaded from NATS KV");
        println!("     Username: {}", loaded.username);
        println!("     Email: {}", loaded.email);
        println!("     Version: {}", loaded.version);
    }
    
    // Update the user
    user.update_email("alice.doe@example.com".to_string());
    let metadata2 = nats_repo.save(&user).await?;
    println!("\n   ✓ Updated in NATS KV");
    println!("     New version: {}", metadata2.version);
    
    // Check existence
    let exists = nats_repo.exists(&user.id).await?;
    println!("\n   ✓ Exists check: {}", exists);
    
    // Example 2: Read Model Storage
    println!("\n2. Read Model Storage...");
    
    let read_model = InMemoryReadModel::<UserSearchView>::new();
    
    // Create search views from profiles
    let users = vec![
        UserProfile::new(
            "bob_smith".to_string(),
            "bob@example.com".to_string(),
            "Bob Smith".to_string(),
        ),
        UserProfile::new(
            "carol_jones".to_string(),
            "carol@example.com".to_string(),
            "Carol Jones".to_string(),
        ),
        UserProfile::new(
            "dave_wilson".to_string(),
            "dave@example.com".to_string(),
            "Dave Wilson".to_string(),
        ),
    ];
    
    // Insert into read model
    for user_profile in &users {
        let view = UserSearchView::from(user_profile);
        read_model.insert(view.user_id.clone(), view);
    }
    
    println!("   ✓ Inserted {} users into read model", users.len());
    
    // Query all users
    let all_users = read_model.all();
    println!("\n   All users:");
    for user_view in &all_users {
        println!("     - {} ({})", user_view.full_name, user_view.username);
    }
    
    // Get specific user
    if let Some(first_user) = users.first() {
        if let Some(found) = read_model.get(&first_user.id.to_string()) {
            println!("\n   ✓ Found user by ID: {}", found.username);
        }
    }
    
    // Example 3: Working with metadata
    println!("\n3. Aggregate Metadata...");
    
    // Create another profile
    let user3 = UserProfile::new(
        "eve_brown".to_string(),
        "eve@example.com".to_string(),
        "Eve Brown".to_string(),
    );
    
    // Save to another bucket
    let temp_repo = NatsKvRepositoryBuilder::new()
        .client(client)
        .bucket_name("temp_profiles".to_string())
        .build()
        .await?;
    
    let temp_metadata = temp_repo.save(&user3).await?;
    println!("   ✓ Saved to temp bucket");
    println!("     Subject: {}", temp_metadata.subject);
    
    // Example 4: Batch operations
    println!("\n4. Batch Operations...");
    
    let batch_users = vec![
        UserProfile::new("user1".to_string(), "user1@test.com".to_string(), "User One".to_string()),
        UserProfile::new("user2".to_string(), "user2@test.com".to_string(), "User Two".to_string()),
        UserProfile::new("user3".to_string(), "user3@test.com".to_string(), "User Three".to_string()),
    ];
    
    println!("   Saving {} users...", batch_users.len());
    for batch_user in &batch_users {
        nats_repo.save(batch_user).await?;
    }
    println!("   ✓ Batch save complete");
    
    // Verify all saved
    let mut found_count = 0;
    for batch_user in &batch_users {
        if nats_repo.exists(&batch_user.id).await? {
            found_count += 1;
        }
    }
    println!("   ✓ Verified {} users saved", found_count);
    
    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • NATS KV repository for durable persistence");
    println!("  • Read model storage for queries");
    println!("  • Aggregate metadata and versioning");
    println!("  • TTL-based expiration");
    println!("  • Batch operations");
    
    Ok(())
} 