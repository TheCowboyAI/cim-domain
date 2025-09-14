use uuid::Uuid;
use std::thread;
use std::time::Duration;

fn main() {
    println!("Testing UUID v7 time ordering...\n");
    
    // Generate several UUID v7s with small delays
    let mut v7_ids = Vec::new();
    
    for i in 0..5 {
        let id = Uuid::now_v7();
        println!("UUID v7 #{}: {}", i, id);
        v7_ids.push(id);
        
        // Small delay to ensure timestamp changes
        thread::sleep(Duration::from_millis(10));
    }
    
    println!("\nVerifying chronological order...");
    
    // Check if they're in ascending order
    let id_strings: Vec<String> = v7_ids.iter().map(|id| id.to_string()).collect();
    for i in 1..id_strings.len() {
        if id_strings[i] > id_strings[i-1] {
            println!("✓ ID {} > ID {} (chronologically ordered)", i, i-1);
        } else {
            println!("✗ ID {} NOT > ID {} (not chronologically ordered!)", i, i-1);
        }
    }
    
    println!("\nComparing with UUID v4 (random)...");
    
    // Generate some v4 IDs that won't be time-ordered
    let mut v4_ids = Vec::new();
    for i in 0..3 {
        let id = Uuid::new_v4();
        println!("UUID v4 #{}: {}", i, id);
        v4_ids.push(id);
    }
    
    println!("\nKey differences:");
    println!("- UUID v7: Time-ordered, sortable, better for indexes");
    println!("- UUID v4: Random, unpredictable, better for security tokens");
    println!("- With vector clocks: v7 gives indexing benefits without clock skew issues");
}