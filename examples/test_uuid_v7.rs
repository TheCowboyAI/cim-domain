use cim_domain::EntityId;
use std::thread;
use std::time::Duration;

/// Test that UUID v7 generates time-ordered IDs
fn main() {
    println!("Testing UUID v7 time ordering...\n");
    
    // Generate several IDs with small delays
    let mut ids = Vec::new();
    
    for i in 0..5 {
        let id = EntityId::<String>::new();
        println!("ID {}: {}", i, id);
        ids.push(id);
        
        // Small delay to ensure timestamp changes
        thread::sleep(Duration::from_millis(10));
    }
    
    println!("\nVerifying chronological order...");
    
    // Convert to strings for comparison
    let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    
    // Check if they're in ascending order
    for i in 1..id_strings.len() {
        if id_strings[i] > id_strings[i-1] {
            println!("✓ ID {} > ID {} (chronologically ordered)", i, i-1);
        } else {
            println!("✗ ID {} NOT > ID {} (not chronologically ordered!)", i, i-1);
        }
    }
    
    println!("\nTesting random ID generation (v4)...");
    
    // Generate some random IDs that won't be time-ordered
    let mut random_ids = Vec::new();
    for i in 0..3 {
        let id = EntityId::<String>::new_random();
        println!("Random ID {}: {}", i, id);
        random_ids.push(id);
    }
    
    println!("\nKey benefits of UUID v7:");
    println!("- Natural chronological sorting");
    println!("- Better database index performance");
    println!("- Built-in millisecond timestamp");
    println!("- Works with vector clocks for causality");
}