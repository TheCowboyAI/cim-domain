// Copyright 2025 Cowboy AI, LLC.

//! Saga as Aggregate-of-Aggregates (pure, FP-oriented)
//!
//! A Saga in this library is modeled exactly like an Aggregate, except its
//! "entities" are other Aggregates (potentially from different domains).
//! The Saga's AggregateRoot is another Aggregate, and causal ordering between
//! the root and participants is determined by a VectorClock. No time is
//! generated here; callers supply any physical time when needed downstream.

use crate::{
    vector_clock::{ActorId, ClockCmp, VectorClock},
    AggregateId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Reference to a participant Aggregate within a Saga.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Participant {
    /// The aggregate identifier.
    pub id: AggregateId,
    /// Optional domain label for human-readable context.
    pub domain: Option<String>,
}

/// Saga modeled as an aggregate-of-aggregates with a vector clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Saga {
    /// The aggregate that acts as the root/coordinator of the saga.
    pub root: Participant,
    /// Other aggregates involved in the saga (may be from different domains).
    pub participants: Vec<Participant>,
    /// Vector clock used for causal ordering between root and participants.
    pub clock: VectorClock,
}

impl Saga {
    /// Create a new Saga with a root aggregate.
    pub fn new(root: Participant) -> Self {
        Self {
            root,
            participants: Vec::new(),
            clock: VectorClock::new(),
        }
    }

    /// Add a participant aggregate (idempotent by `id`). Returns a new Saga.
    pub fn with_participant(mut self, p: Participant) -> Self {
        if !self.participants.iter().any(|x| x.id == p.id) {
            self.participants.push(p);
        }
        self
    }

    /// Advance the saga's vector clock for the given actor (pure increment).
    pub fn tick(&self, actor: impl Into<ActorId>) -> Self {
        let clock = self.clock.increment(actor);
        Self {
            clock,
            ..self.clone()
        }
    }

    /// Merge this saga's clock with another, returning a new Saga carrying
    /// the merged clock. Participants/root are taken from `self` unchanged.
    pub fn merge_clock(&self, other_clock: &VectorClock) -> Self {
        let clock = self.clock.merge(other_clock);
        Self {
            clock,
            ..self.clone()
        }
    }

    /// Partial order between this saga and another via vector clocks.
    pub fn order(&self, other: &Saga) -> ClockCmp {
        self.clock.compare(&other.clock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_participant() -> Participant {
        Participant {
            id: AggregateId::new(),
            domain: Some("orders".into()),
        }
    }

    #[test]
    fn saga_is_aggregate_of_aggregates() {
        let root = mk_participant();
        let saga = Saga::new(root.clone())
            .with_participant(Participant {
                id: AggregateId::new(),
                domain: Some("payments".into()),
            })
            .with_participant(Participant {
                id: AggregateId::new(),
                domain: Some("shipping".into()),
            });

        assert_eq!(saga.root, root);
        assert!(saga.participants.len() >= 2);
        assert_eq!(saga.clock, VectorClock::new());
    }

    #[test]
    fn vector_clock_orders_root_and_participants() {
        let root = mk_participant();
        let mut s1 = Saga::new(root.clone());
        let s2 = Saga::new(root);

        // Root actor ID can be domain-scoped or aggregate-scoped; here we use a simple label.
        s1 = s1.tick("root"); // root advances
        assert_eq!(s1.order(&s2), ClockCmp::After);
        assert_eq!(s2.order(&s1), ClockCmp::Before);

        // Concurrent updates from different actors produce Concurrent order.
        let a = s1.clone().tick("payments");
        let b = s1.clone().tick("shipping");
        assert_eq!(a.order(&b), ClockCmp::Concurrent);

        // Merging reconciles causality without physical time.
        let merged = a.merge_clock(&b.clock);
        assert_eq!(merged.clock.get("payments"), 1);
        assert_eq!(merged.clock.get("shipping"), 1);
    }
}
