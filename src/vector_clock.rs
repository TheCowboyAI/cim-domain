// Copyright 2025 Cowboy AI, LLC.

//! Vector clocks for causal ordering (pure, no time generation).
//!
//! This module provides a functional, immutable vector clock implementation
//! suitable for sagas and causal reasoning. It never generates physical time;
//! callers supply all values and identifiers. All operations return new values
//! without side effects.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Identifier for a logical actor (process, saga, participant).
pub type ActorId = String;

/// Partial order relationship between two vector clocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ClockCmp {
    /// All counters are equal.
    Equal,
    /// Self is causally before other (self <= other and self != other).
    Before,
    /// Self is causally after other (self >= other and self != other).
    After,
    /// Neither before nor after: concurrent updates.
    Concurrent,
}

/// Immutable vector clock.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VectorClock {
    /// Logical counters per actor.
    pub counters: HashMap<ActorId, u64>,
}

impl VectorClock {
    /// Create an empty vector clock.
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Create a vector clock from a map of counters.
    pub fn from_map(counters: HashMap<ActorId, u64>) -> Self {
        Self { counters }
    }

    /// Get the counter for an actor (0 if missing).
    pub fn get(&self, actor: &str) -> u64 {
        *self.counters.get(actor).unwrap_or(&0)
    }

    /// Return a new clock with the actor's counter incremented by 1.
    pub fn increment(&self, actor: impl Into<ActorId>) -> Self {
        let actor = actor.into();
        let mut next = self.counters.clone();
        let entry = next.entry(actor).or_insert(0);
        *entry = entry.saturating_add(1);
        Self { counters: next }
    }

    /// Merge two clocks by taking element-wise maxima (least upper bound).
    pub fn merge(&self, other: &Self) -> Self {
        let mut merged = self.counters.clone();
        for (actor, &count) in &other.counters {
            let entry = merged.entry(actor.clone()).or_insert(0);
            if count > *entry {
                *entry = count;
            }
        }
        Self { counters: merged }
    }

    /// Partial order comparison per vector clock semantics.
    pub fn compare(&self, other: &Self) -> ClockCmp {
        let mut le = true; // self <= other
        let mut ge = true; // self >= other

        // Union of keys
        for key in self.counters.keys().chain(other.counters.keys()) {
            let a = self.get(key);
            let b = other.get(key);
            if a > b {
                le = false;
            }
            if a < b {
                ge = false;
            }
        }

        match (le, ge) {
            (true, true) => ClockCmp::Equal,
            (true, false) => ClockCmp::Before,
            (false, true) => ClockCmp::After,
            (false, false) => ClockCmp::Concurrent,
        }
    }

    /// Return a standard partial_cmp: Some(Ordering) if comparable, None if concurrent.
    pub fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.compare(other) {
            ClockCmp::Equal => Some(Ordering::Equal),
            ClockCmp::Before => Some(Ordering::Less),
            ClockCmp::After => Some(Ordering::Greater),
            ClockCmp::Concurrent => None,
        }
    }

    /// True if self causally dominates other (self >= other and !=).
    pub fn dominates(&self, other: &Self) -> bool {
        matches!(self.compare(other), ClockCmp::After)
    }

    /// True if self is causally dominated by other (self <= other and !=).
    pub fn is_dominated_by(&self, other: &Self) -> bool {
        matches!(self.compare(other), ClockCmp::Before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_and_get() {
        let vc = VectorClock::new();
        let vc1 = vc.increment("a");
        let vc2 = vc1.increment("a");
        assert_eq!(vc.get("a"), 0);
        assert_eq!(vc1.get("a"), 1);
        assert_eq!(vc2.get("a"), 2);
    }

    #[test]
    fn test_merge_and_compare() {
        let a1 = VectorClock::new().increment("a");
        let b1 = VectorClock::new().increment("b");

        let merged = a1.merge(&b1);
        assert_eq!(merged.get("a"), 1);
        assert_eq!(merged.get("b"), 1);

        assert_eq!(a1.compare(&merged), ClockCmp::Before);
        assert_eq!(b1.compare(&merged), ClockCmp::Before);
        assert_eq!(a1.compare(&b1), ClockCmp::Concurrent);
        assert_eq!(merged.compare(&merged), ClockCmp::Equal);
    }

    #[test]
    fn test_partial_cmp() {
        let a = VectorClock::new().increment("x");
        let b = a.merge(&VectorClock::new().increment("y"));
        assert_eq!(a.partial_cmp(&b), Some(Ordering::Less));
        assert_eq!(b.partial_cmp(&a), Some(Ordering::Greater));

        let c = VectorClock::new().increment("z");
        assert_eq!(a.partial_cmp(&c), None); // concurrent
    }
}
