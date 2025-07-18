// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating event replay functionality
//!
//! This example shows:
//! - Replaying events from an event store
//! - Building aggregate state from events
//! - Event handlers for replay
//! - Snapshot and replay optimization

use cim_domain::{
    // Core types
    EntityId,
    markers::AggregateMarker,
    AggregateRoot,
    
    // Events
    DomainEventEnum,
    WorkflowStarted, WorkflowTransitionExecuted,
    
    // Infrastructure
    infrastructure::{
        EventStore,
        event_store::{StoredEvent, EventMetadata},
        event_replay::{EventHandler, ReplayError, ReplayStats},
        jetstream_event_store::{JetStreamEventStore, JetStreamConfig},
    },
    
    // IDs
    WorkflowId, GraphId,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

/// Example aggregate that we'll rebuild from events
#[derive(Debug, Clone)]
struct Account {
    id: EntityId<AggregateMarker>,
    owner: String,
    balance: f64,
    transactions: Vec<Transaction>,
    status: AccountStatus,
    version: u64,
}

#[derive(Debug, Clone)]
struct Transaction {
    id: Uuid,
    amount: f64,
    transaction_type: TransactionType,
    timestamp: DateTime<Utc>,
    description: String,
}

#[derive(Debug, Clone, PartialEq)]
enum TransactionType {
    Deposit,
    Withdrawal,
    Transfer,
}

#[derive(Debug, Clone, PartialEq)]
enum AccountStatus {
    Active,
    Frozen,
    Closed,
}

impl AggregateRoot for Account {
    type Id = EntityId<AggregateMarker>;
    
    fn id(&self) -> Self::Id {
        self.id
    }
    
    fn version(&self) -> u64 {
        self.version
    }
    
    fn increment_version(&mut self) {
        self.version += 1;
    }
}

/// Event handler that rebuilds account state from events
struct AccountEventHandler {
    accounts: HashMap<String, Account>,
    event_count: usize,
    last_event_time: Option<DateTime<Utc>>,
}

impl AccountEventHandler {
    fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            event_count: 0,
            last_event_time: None,
        }
    }
    
    fn get_account(&self, id: &str) -> Option<&Account> {
        self.accounts.get(id)
    }
    
    fn process_workflow_event(&mut self, event: &DomainEventEnum, aggregate_id: &str) -> Result<(), ReplayError> {
        match event {
            DomainEventEnum::WorkflowStarted(e) => {
                // Interpret as account creation
                let account = Account {
                    id: EntityId::new(),
                    owner: e.initial_state.clone(), // Using initial_state as owner name for demo
                    balance: 0.0,
                    transactions: Vec::new(),
                    status: AccountStatus::Active,
                    version: 1,
                };
                self.accounts.insert(aggregate_id.to_string(), account);
                println!("      Created account: {}", aggregate_id);
            }
            
            DomainEventEnum::WorkflowTransitionExecuted(e) => {
                // Interpret as transaction
                if let Some(account) = self.accounts.get_mut(aggregate_id) {
                    // Parse transaction from the event
                    if let Some(amount) = e.input.get("amount").and_then(|v| v.as_f64()) {
                        let transaction_type = match e.to_state.as_str() {
                            "deposited" => TransactionType::Deposit,
                            "withdrawn" => TransactionType::Withdrawal,
                            _ => TransactionType::Transfer,
                        };
                        
                        let transaction = Transaction {
                            id: Uuid::new_v4(),
                            amount,
                            transaction_type: transaction_type.clone(),
                            timestamp: e.executed_at,
                            description: e.input.get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Transaction")
                                .to_string(),
                        };
                        
                        // Update balance
                        match transaction_type {
                            TransactionType::Deposit => account.balance += amount,
                            TransactionType::Withdrawal => account.balance -= amount,
                            TransactionType::Transfer => {} // Handled separately
                        }
                        
                        // Check if account is frozen
                        if account.status == AccountStatus::Frozen {
                            println!("      Warning: Transaction on frozen account!");
                        }
                        
                        println!("      Transaction ID: {}", transaction.id);
                        println!("      Timestamp: {}", transaction.timestamp);
                        println!("      Description: {}", transaction.description);
                        
                        account.transactions.push(transaction);
                        account.increment_version();
                        
                        println!("      Processed transaction: {} {}", 
                            if amount >= 0.0 { "+" } else { "" }, 
                            amount
                        );
                    }
                }
            }
            
            DomainEventEnum::WorkflowCompleted(_) => {
                // Interpret as account closure
                if let Some(account) = self.accounts.get_mut(aggregate_id) {
                    account.status = AccountStatus::Closed;
                    account.increment_version();
                    println!("      Closed account");
                }
            }
            
            DomainEventEnum::WorkflowSuspended(_) => {
                // Interpret as account freeze
                if let Some(account) = self.accounts.get_mut(aggregate_id) {
                    account.status = AccountStatus::Frozen;
                    account.increment_version();
                    println!("      Froze account");
                }
            }
            
            _ => {} // Ignore other events
        }
        
        Ok(())
    }
}

#[async_trait]
impl EventHandler for AccountEventHandler {
    async fn handle_event(&mut self, event: &StoredEvent) -> Result<(), ReplayError> {
        self.event_count += 1;
        self.last_event_time = Some(event.stored_at);
        
        // Process the domain event
        self.process_workflow_event(&event.event, &event.aggregate_id)?;
        
        Ok(())
    }
    
    async fn on_replay_start(&mut self) -> Result<(), ReplayError> {
        println!("   Starting event replay...");
        self.event_count = 0;
        self.last_event_time = None;
        Ok(())
    }
    
    async fn on_replay_complete(&mut self, stats: &ReplayStats) -> Result<(), ReplayError> {
        println!("   Replay complete!");
        println!("   Total events processed: {}", stats.events_processed);
        println!("   Duration: {}ms", stats.duration_ms);
        println!("   Events/second: {:.2}", stats.events_per_second);
        if let Some(last_time) = self.last_event_time {
            println!("   Last event time: {}", last_time);
        }
        Ok(())
    }
}

/// Helper to create test events
fn create_test_events(_account_id: &str) -> Vec<DomainEventEnum> {
    let workflow_id = WorkflowId::new();
    let definition_id = GraphId::new();
    
    vec![
        // Account creation
        DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id: workflow_id.clone(),
            definition_id: definition_id.clone(),
            initial_state: "John Doe".to_string(), // Owner name
            started_at: Utc::now(),
        }),
        
        // Deposit
        DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
            workflow_id: workflow_id.clone(),
            from_state: "created".to_string(),
            to_state: "deposited".to_string(),
            input: json!({
                "amount": 1000.0,
                "description": "Initial deposit"
            }),
            output: json!({"success": true}),
            executed_at: Utc::now(),
        }),
        
        // Withdrawal
        DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
            workflow_id: workflow_id.clone(),
            from_state: "active".to_string(),
            to_state: "withdrawn".to_string(),
            input: json!({
                "amount": 250.0,
                "description": "ATM withdrawal"
            }),
            output: json!({"success": true}),
            executed_at: Utc::now(),
        }),
        
        // Another deposit
        DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
            workflow_id: workflow_id.clone(),
            from_state: "active".to_string(),
            to_state: "deposited".to_string(),
            input: json!({
                "amount": 500.0,
                "description": "Paycheck"
            }),
            output: json!({"success": true}),
            executed_at: Utc::now(),
        }),
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Event Replay Example");
    println!("===================\n");
    
    // Note: This example requires a running NATS server with JetStream enabled
    // Run: docker run -p 4222:4222 nats:latest -js
    
    // Connect to NATS
    println!("1. Setting up event store...");
    let client = async_nats::connect("nats://localhost:4222").await?;
    
    let config = JetStreamConfig {
        stream_name: "replay-demo".to_string(),
        stream_subjects: vec!["events.>".to_string()],
        cache_size: 100,
        subject_prefix: "events".to_string(),
    };
    
    let event_store = JetStreamEventStore::new(client, config).await?;
    println!("   ✓ Event store ready\n");
    
    // Create test account ID
    let account_id = format!("account-{}", Uuid::new_v4());
    
    // Store some events
    println!("2. Storing events...");
    let events = create_test_events(&account_id);
    
    let metadata = EventMetadata {
        correlation_id: Some(Uuid::new_v4().to_string()),
        causation_id: None,
        triggered_by: Some("system".to_string()),
        custom: None,
    };
    
    event_store.append_events(
        &account_id,
        "Account",
        events.clone(),
        None,
        metadata,
    ).await?;
    
    println!("   ✓ Stored {} events\n", events.len());
    
    // Create event handler for replay
    println!("3. Replaying events...");
    let mut handler = AccountEventHandler::new();
    
    // Get all events for the account
    let stored_events = event_store.get_events(&account_id, None).await?;
    
    // Manually replay events (simulating what a replay service would do)
    handler.on_replay_start().await?;
    
    let start_time = std::time::Instant::now();
    for event in &stored_events {
        println!("   Processing event {} (v{})", event.event_type(), event.sequence);
        handler.handle_event(event).await?;
    }
    
    // Create stats for completion
    let duration = start_time.elapsed();
    let stats = ReplayStats {
        events_processed: stored_events.len() as u64,
        aggregates_rebuilt: 1,
        errors: 0,
        duration_ms: duration.as_millis() as u64,
        events_per_second: if duration.as_secs() > 0 {
            stored_events.len() as f64 / duration.as_secs_f64()
        } else {
            stored_events.len() as f64 * 1000.0
        },
    };
    
    handler.on_replay_complete(&stats).await?;
    println!();
    
    // Show rebuilt state
    println!("4. Rebuilt account state:");
    if let Some(account) = handler.get_account(&account_id) {
        println!("   ID: {}", account.id);
        println!("   Owner: {}", account.owner);
        println!("   Balance: ${:.2}", account.balance);
        println!("   Status: {:?}", account.status);
        println!("   Version: {}", account.version);
        println!("   Transactions: {}", account.transactions.len());
        
        println!("\n   Transaction history:");
        for (i, tx) in account.transactions.iter().enumerate() {
            println!("     {}. {:?} ${:.2} - {}", 
                i + 1, 
                tx.transaction_type, 
                tx.amount,
                tx.description
            );
        }
    }
    
    // Demonstrate partial replay
    println!("\n5. Partial replay (from version 2)...");
    let mut partial_handler = AccountEventHandler::new();
    
    // Only replay events after version 2
    let partial_events: Vec<_> = stored_events
        .into_iter()
        .filter(|e| e.sequence > 2)
        .collect();
    
    println!("   Replaying {} events (out of {})", partial_events.len(), events.len());
    
    // First, we need to restore state up to version 2 (in real system, from snapshot)
    // For demo, we'll just create the initial state
    let initial_account = Account {
        id: EntityId::new(),
        owner: "John Doe".to_string(),
        balance: 1000.0, // After first deposit
        transactions: vec![Transaction {
            id: Uuid::new_v4(),
            amount: 1000.0,
            transaction_type: TransactionType::Deposit,
            timestamp: Utc::now(),
            description: "Initial deposit".to_string(),
        }],
        status: AccountStatus::Active,
        version: 2,
    };
    partial_handler.accounts.insert(account_id.clone(), initial_account);
    
    // Now replay remaining events
    for event in partial_events {
        handler.handle_event(&event).await?;
    }
    
    println!("   ✓ Partial replay complete");
    
    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • Replaying events to rebuild aggregate state");
    println!("  • Custom event handlers for replay");
    println!("  • Processing events in sequence");
    println!("  • Partial replay from a specific version");
    println!("  • Building domain objects from event history");
    
    Ok(())
}
