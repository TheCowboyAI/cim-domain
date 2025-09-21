//! Template: BDD test for a domain feature.
//! Copy into `tests/bdd_<feature>.rs` and adjust step handling.

use cim_domain::*;

// 1) Put your Gherkin file under doc/qa/features/<feature>.feature
const FEATURE: &str = include_str!("../../doc/qa/features/<feature>.feature");

// 2) World holds state and any collected events (if your domain emits them)
#[derive(Debug, Clone)]
struct World {
    // Replace with your aggregate state
    // state: YourState,
    events: Vec<Box<dyn DomainEvent>>,
}

impl Default for World {
    fn default() -> Self {
        Self { events: vec![] }
    }
}

// 3) Map inputs (When) to pure domain operations
fn apply(_world: &mut World, _step: &str) {
    // Example: parse command and apply to state machine, collect events
    // let input = ...;
    // let next = world.state.valid_transitions(&input).first().cloned();
    // if let Some(n) = next { let out = world.state.transition_output(&n, &input); world.state = n; world.events.extend(out.to_events()); }
}

#[test]
fn bdd_feature_runs() {
    // Minimal interpreter: route steps by prefix
    for block in FEATURE.split("\n\n").map(str::trim) {
        if block.starts_with("Scenario:") {
            let mut world = World::default();
            for line in block.lines().map(str::trim) {
                match line {
                    // Given steps
                    _ if line.starts_with("Given ") => {
                        // Initialize world
                    }
                    // When steps
                    _ if line.starts_with("When ") || line.starts_with("And we ") => {
                        apply(&mut world, line);
                    }
                    // Then steps
                    _ if line.starts_with("Then ") || line.starts_with("And Expect ") => {
                        // Assert final state and/or expected event stream
                        // use tests::bdd_support::assert_event_types(&world.events, &["EventType"]);
                    }
                    _ => {}
                }
            }
        }
    }
}
