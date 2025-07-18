//! Full Event Sourcing Demo - CIM Architecture
//!
//! This demo shows event sourcing concepts including:
//! - Event generation from domain operations
//! - Event storage and retrieval patterns
//! - Event replay and projection building
//! - CID chain concepts for integrity
//!
//! Note: This demo uses mock infrastructure to demonstrate concepts.
//! In production, you would use actual NATS JetStream or similar.

use cim_domain::{
    // Domain types
    EntityId, DomainEvent, DomainEventEnvelopeWithMetadata,
    EventMetadata, PropagationScope,
    AggregateRoot, CommandEnvelope, Command,
    
    // CQRS types
    CommandAcknowledgment, CommandStatus, CorrelationId, IdType,
    
    // Infrastructure types for demo
    EventHandler as ReplayEventHandler,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use cid::Cid;

// Define a simple aggregate for demo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BankAccountAggregate;

impl AggregateRoot for BankAccountAggregate {
    type Id = EntityId<BankAccountAggregate>;
    
    fn id(&self) -> Self::Id {
        EntityId::new()
    }
    
    fn version(&self) -> u64 {
        1
    }
    
    fn increment_version(&mut self) {}
}

// Define domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
enum AccountEvent {
    AccountOpened {
        account_id: String,
        owner_name: String,
        initial_balance: f64,
    },
    MoneyDeposited {
        account_id: String,
        amount: f64,
        balance_after: f64,
    },
    MoneyWithdrawn {
        account_id: String,
        amount: f64,
        balance_after: f64,
    },
    AccountClosed {
        account_id: String,
        final_balance: f64,
    },
}

impl DomainEvent for AccountEvent {
    fn subject(&self) -> String {
        match self {
            AccountEvent::AccountOpened { .. } => "bank.account.opened.v1",
            AccountEvent::MoneyDeposited { .. } => "bank.account.deposited.v1",
            AccountEvent::MoneyWithdrawn { .. } => "bank.account.withdrawn.v1",
            AccountEvent::AccountClosed { .. } => "bank.account.closed.v1",
        }.to_string()
    }
    
    fn aggregate_id(&self) -> uuid::Uuid {
        // In real implementation, parse from account_id
        uuid::Uuid::new_v4()
    }
    
    fn event_type(&self) -> &'static str {
        match self {
            AccountEvent::AccountOpened { .. } => "AccountOpened",
            AccountEvent::MoneyDeposited { .. } => "MoneyDeposited",
            AccountEvent::MoneyWithdrawn { .. } => "MoneyWithdrawn",
            AccountEvent::AccountClosed { .. } => "AccountClosed",
        }
    }
}

// Define commands
#[derive(Debug, Clone, Serialize, Deserialize)]
enum AccountCommand {
    OpenAccount { owner_name: String, initial_deposit: f64 },
    Deposit { amount: f64 },
    Withdraw { amount: f64 },
    CloseAccount,
}

impl Command for AccountCommand {
    type Aggregate = BankAccountAggregate;
    
    fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
        None // For demo simplicity
    }
}

// Mock event store
#[derive(Debug)]
struct MockEventStore {
    events: Arc<Mutex<Vec<StoredEvent>>>,
    by_aggregate: Arc<Mutex<HashMap<String, Vec<usize>>>>,
}

#[derive(Debug, Clone)]
struct StoredEvent {
    event_id: Cid,
    aggregate_id: String,
    sequence: u64,
    event: Box<AccountEvent>,
    metadata: EventMetadata,
    stored_at: chrono::DateTime<chrono::Utc>,
}

impl MockEventStore {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            by_aggregate: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn append_events(&self, aggregate_id: &str, events: Vec<AccountEvent>) -> Vec<Cid> {
        let mut event_store = self.events.lock().unwrap();
        let mut by_aggregate = self.by_aggregate.lock().unwrap();
        
        let aggregate_events = by_aggregate.entry(aggregate_id.to_string()).or_default();
        let mut event_ids = Vec::new();
        
        for event in events {
            let sequence = aggregate_events.len() as u64 + 1;
            let event_id = Cid::default(); // In real implementation, calculate from content
            
            let stored = StoredEvent {
                event_id,
                aggregate_id: aggregate_id.to_string(),
                sequence,
                event: Box::new(event),
                metadata: EventMetadata {
                    source: "demo".to_string(),
                    version: "v1".to_string(),
                    propagation_scope: PropagationScope::LocalOnly,
                    properties: HashMap::new(),
                },
                stored_at: Utc::now(),
            };
            
            let index = event_store.len();
            event_store.push(stored);
            aggregate_events.push(index);
            event_ids.push(event_id);
        }
        
        event_ids
    }
    
    fn get_events(&self, aggregate_id: &str) -> Vec<StoredEvent> {
        let event_store = self.events.lock().unwrap();
        let by_aggregate = self.by_aggregate.lock().unwrap();
        
        if let Some(indices) = by_aggregate.get(aggregate_id) {
            indices.iter()
                .filter_map(|&idx| event_store.get(idx).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    fn get_all_events(&self) -> Vec<StoredEvent> {
        self.events.lock().unwrap().clone()
    }
}

// Bank account aggregate state
#[derive(Debug, Clone)]
struct BankAccount {
    id: String,
    owner: String,
    balance: f64,
    is_closed: bool,
    version: u64,
}

impl BankAccount {
    fn new(id: String, owner: String, initial_balance: f64) -> Self {
        Self {
            id,
            owner,
            balance: initial_balance,
            is_closed: false,
            version: 0,
        }
    }
    
    fn apply_event(&mut self, event: &AccountEvent) {
        match event {
            AccountEvent::AccountOpened { owner_name, initial_balance, .. } => {
                self.owner = owner_name.clone();
                self.balance = *initial_balance;
            }
            AccountEvent::MoneyDeposited { balance_after, .. } => {
                self.balance = *balance_after;
            }
            AccountEvent::MoneyWithdrawn { balance_after, .. } => {
                self.balance = *balance_after;
            }
            AccountEvent::AccountClosed { .. } => {
                self.is_closed = true;
            }
        }
        self.version += 1;
    }
    
    fn handle_command(&self, command: AccountCommand) -> Result<Vec<AccountEvent>, String> {
        if self.is_closed {
            return Err("Account is closed".to_string());
        }
        
        match command {
            AccountCommand::OpenAccount { .. } => {
                Err("Account already exists".to_string())
            }
            AccountCommand::Deposit { amount } => {
                if amount <= 0.0 {
                    Err("Amount must be positive".to_string())
                } else {
                    Ok(vec![AccountEvent::MoneyDeposited {
                        account_id: self.id.clone(),
                        amount,
                        balance_after: self.balance + amount,
                    }])
                }
            }
            AccountCommand::Withdraw { amount } => {
                if amount <= 0.0 {
                    Err("Amount must be positive".to_string())
                } else if amount > self.balance {
                    Err("Insufficient funds".to_string())
                } else {
                    Ok(vec![AccountEvent::MoneyWithdrawn {
                        account_id: self.id.clone(),
                        amount,
                        balance_after: self.balance - amount,
                    }])
                }
            }
            AccountCommand::CloseAccount => {
                if self.balance > 0.0 {
                    Err("Cannot close account with positive balance".to_string())
                } else {
                    Ok(vec![AccountEvent::AccountClosed {
                        account_id: self.id.clone(),
                        final_balance: self.balance,
                    }])
                }
            }
        }
    }
}

// Projection for account summaries
#[derive(Debug, Default)]
struct AccountSummaryProjection {
    accounts: HashMap<String, AccountSummary>,
    total_deposits: f64,
    total_withdrawals: f64,
}

#[derive(Debug, Clone)]
struct AccountSummary {
    id: String,
    owner: String,
    balance: f64,
    is_active: bool,
    transaction_count: u32,
}

impl AccountSummaryProjection {
    fn handle_event(&mut self, event: &AccountEvent) {
        match event {
            AccountEvent::AccountOpened { account_id, owner_name, initial_balance } => {
                self.accounts.insert(account_id.clone(), AccountSummary {
                    id: account_id.clone(),
                    owner: owner_name.clone(),
                    balance: *initial_balance,
                    is_active: true,
                    transaction_count: 0,
                });
                self.total_deposits += initial_balance;
            }
            AccountEvent::MoneyDeposited { account_id, amount, balance_after } => {
                if let Some(account) = self.accounts.get_mut(account_id) {
                    account.balance = *balance_after;
                    account.transaction_count += 1;
                    self.total_deposits += amount;
                }
            }
            AccountEvent::MoneyWithdrawn { account_id, amount, balance_after } => {
                if let Some(account) = self.accounts.get_mut(account_id) {
                    account.balance = *balance_after;
                    account.transaction_count += 1;
                    self.total_withdrawals += amount;
                }
            }
            AccountEvent::AccountClosed { account_id, .. } => {
                if let Some(account) = self.accounts.get_mut(account_id) {
                    account.is_active = false;
                }
            }
        }
    }
}

fn main() {
    println!("=== CIM Event Sourcing Demo ===\n");
    
    // Initialize event store
    let event_store = MockEventStore::new();
    
    // 1. Create new account
    println!("1️⃣ Creating New Account\n");
    
    let account_id = "ACC-001";
    let open_command = AccountCommand::OpenAccount {
        owner_name: "Alice Smith".to_string(),
        initial_deposit: 1000.0,
    };
    
    // Handle command to generate events
    let events = vec![AccountEvent::AccountOpened {
        account_id: account_id.to_string(),
        owner_name: "Alice Smith".to_string(),
        initial_balance: 1000.0,
    }];
    
    // Store events
    let event_ids = event_store.append_events(account_id, events.clone());
    println!("   ✅ Account opened for Alice Smith");
    println!("   Initial balance: $1000.00");
    println!("   Event CID: {:?}", event_ids[0]);
    
    // 2. Process transactions
    println!("\n2️⃣ Processing Transactions\n");
    
    // Rebuild aggregate from events
    let mut account = BankAccount::new(account_id.to_string(), String::new(), 0.0);
    let stored_events = event_store.get_events(account_id);
    
    for stored in &stored_events {
        account.apply_event(&stored.event);
    }
    
    // Deposit
    let deposit_cmd = AccountCommand::Deposit { amount: 500.0 };
    match account.handle_command(deposit_cmd) {
        Ok(events) => {
            event_store.append_events(account_id, events.clone());
            for event in &events {
                account.apply_event(event);
            }
            println!("   ✅ Deposited $500.00");
            println!("   New balance: ${:.2}", account.balance);
        }
        Err(e) => println!("   ❌ Deposit failed: {}", e),
    }
    
    // Withdraw
    let withdraw_cmd = AccountCommand::Withdraw { amount: 200.0 };
    match account.handle_command(withdraw_cmd) {
        Ok(events) => {
            event_store.append_events(account_id, events.clone());
            for event in &events {
                account.apply_event(event);
            }
            println!("   ✅ Withdrew $200.00");
            println!("   New balance: ${:.2}", account.balance);
        }
        Err(e) => println!("   ❌ Withdrawal failed: {}", e),
    }
    
    // Try invalid withdrawal
    let invalid_withdraw = AccountCommand::Withdraw { amount: 2000.0 };
    match account.handle_command(invalid_withdraw) {
        Ok(_) => println!("   Unexpected success!"),
        Err(e) => println!("   ❌ Expected error: {}", e),
    }
    
    // 3. Show event history
    println!("\n3️⃣ Event History\n");
    
    let all_events = event_store.get_events(account_id);
    println!("   Account {} has {} events:", account_id, all_events.len());
    for (i, stored) in all_events.iter().enumerate() {
        println!("   {}. {} (seq: {})", 
            i + 1, 
            stored.event.event_type(),
            stored.sequence
        );
    }
    
    // 4. Build projection
    println!("\n4️⃣ Building Projections\n");
    
    let mut projection = AccountSummaryProjection::default();
    
    // Replay all events
    let all_stored = event_store.get_all_events();
    for stored in &all_stored {
        projection.handle_event(&stored.event);
    }
    
    println!("   Projection Summary:");
    println!("   - Total accounts: {}", projection.accounts.len());
    println!("   - Total deposits: ${:.2}", projection.total_deposits);
    println!("   - Total withdrawals: ${:.2}", projection.total_withdrawals);
    println!("   - Net flow: ${:.2}", projection.total_deposits - projection.total_withdrawals);
    
    for (id, summary) in &projection.accounts {
        println!("\n   Account: {}", id);
        println!("   - Owner: {}", summary.owner);
        println!("   - Balance: ${:.2}", summary.balance);
        println!("   - Status: {}", if summary.is_active { "Active" } else { "Closed" });
        println!("   - Transactions: {}", summary.transaction_count);
    }
    
    // 5. Demonstrate aggregate rebuilding
    println!("\n5️⃣ Aggregate Rebuilding\n");
    
    // Rebuild from scratch
    let mut rebuilt_account = BankAccount::new(account_id.to_string(), String::new(), 0.0);
    let events = event_store.get_events(account_id);
    
    println!("   Replaying {} events...", events.len());
    for stored in &events {
        rebuilt_account.apply_event(&stored.event);
    }
    
    println!("   ✅ Account rebuilt:");
    println!("   - Owner: {}", rebuilt_account.owner);
    println!("   - Balance: ${:.2}", rebuilt_account.balance);
    println!("   - Version: {}", rebuilt_account.version);
    println!("   - Status: {}", if rebuilt_account.is_closed { "Closed" } else { "Active" });
    
    // 6. Event sourcing benefits
    println!("\n6️⃣ Event Sourcing Benefits\n");
    
    println!("   📝 Complete Audit Trail:");
    println!("      Every state change is recorded as an event");
    
    println!("\n   ⏮️ Time Travel:");
    println!("      Can rebuild state at any point in time");
    
    println!("\n   📊 Multiple Projections:");
    println!("      Different views from same event stream");
    
    println!("\n   🔍 Event Analysis:");
    println!("      Business intelligence from event patterns");
    
    println!("\n   🔗 CID Chains:");
    println!("      Content-addressed storage ensures integrity");
    
    // 7. Summary
    println!("\n✅ Event Sourcing Demo Complete!\n");
    
    println!("💡 Key Concepts Demonstrated:");
    println!("   - Commands generate events");
    println!("   - Events are immutable facts");
    println!("   - State is rebuilt from events");
    println!("   - Projections provide read models");
    println!("   - CID chains ensure integrity");
    println!("   - Complete audit trail maintained");
}