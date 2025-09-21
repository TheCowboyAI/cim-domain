// Copyright 2025 Cowboy AI, LLC.

//! Transaction State Machine (Mealy) — Graph with Rules
//!
//! Transactions are modeled as a Mealy state machine: transitions are edges in
//! a graph, and rules (guards) determine allowed moves based on inputs. This is
//! not a procedural step list; it is a graph with explicit morphisms.

use crate::state_machine::{MealyStateTransitions, State, TransitionInput, TransitionOutput};
use crate::DomainEvent;
use serde::{Deserialize, Serialize};

/// States for a generic transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionState {
    /// No active transaction (terminal for single morphisms)
    Idle,
    /// Transaction was started; awaiting validation or application
    Started,
    /// Changes validated and applied to in-memory state
    Applied,
    /// Transaction committed (terminal)
    Committed,
    /// Transaction cancelled/aborted (terminal)
    Cancelled,
    /// Transaction failed (terminal)
    Failed,
}

impl State for TransactionState {
    fn name(&self) -> &'static str {
        match self {
            TransactionState::Idle => "Idle",
            TransactionState::Started => "Started",
            TransactionState::Applied => "Applied",
            TransactionState::Committed => "Committed",
            TransactionState::Cancelled => "Cancelled",
            TransactionState::Failed => "Failed",
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(
            self,
            TransactionState::Idle
                | TransactionState::Committed
                | TransactionState::Cancelled
                | TransactionState::Failed
        )
    }
}

/// Inputs that drive transaction transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionInput {
    /// Aggregate starts a new transaction
    Start,
    /// Validation passed; apply changes
    ValidateOk,
    /// Validation failed
    ValidateFail,
    /// Commit the transaction
    Commit,
    /// Cancel/abort
    Cancel,
}

impl TransitionInput for TransactionInput {
    fn description(&self) -> String {
        format!("{self:?}")
    }
}

/// Output for transaction transitions (wraps DomainEvent vectors if needed).
#[derive(Debug, Default)]
pub struct TxOutput {
    /// Events produced by transition (optional)
    pub events: Vec<Box<dyn DomainEvent>>,
}

// Note: We cannot derive Clone because `Box<dyn DomainEvent>` is not clonable.
// Cloning a TxOutput semantically represents copying the output metadata but not
// duplicating event payloads (which are meant to be consumed). Therefore, the
// cloned value contains an empty events vector.
impl Clone for TxOutput {
    fn clone(&self) -> Self {
        TxOutput { events: Vec::new() }
    }
}

impl TransitionOutput for TxOutput {
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
        // Outputs are typically consumed; return empty to avoid cloning trait objects
        Vec::new()
    }
}

impl MealyStateTransitions for TransactionState {
    type Input = TransactionInput;
    type Output = TxOutput;

    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool {
        use TransactionInput as I;
        use TransactionState as S;
        matches!(
            (*self, target, input),
            (S::Idle, S::Started, I::Start)
                | (S::Started, S::Applied, I::ValidateOk)
                | (S::Started, S::Failed, I::ValidateFail)
                | (S::Applied, S::Committed, I::Commit)
                | (S::Started, S::Cancelled, I::Cancel)
                | (S::Applied, S::Cancelled, I::Cancel)
        )
    }

    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self> {
        use TransactionInput as I;
        use TransactionState as S;
        match (*self, input) {
            (S::Idle, I::Start) => vec![S::Started],
            (S::Started, I::ValidateOk) => vec![S::Applied],
            (S::Started, I::ValidateFail) => vec![S::Failed],
            (S::Applied, I::Commit) => vec![S::Committed],
            (S::Started, I::Cancel) => vec![S::Cancelled],
            (S::Applied, I::Cancel) => vec![S::Cancelled],
            _ => Vec::new(),
        }
    }

    fn transition_output(&self, _target: &Self, _input: &Self::Input) -> Self::Output {
        TxOutput::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_terminal() {
        assert!(TransactionState::Idle.is_terminal());
        assert!(TransactionState::Committed.is_terminal());
        assert!(TransactionState::Cancelled.is_terminal());
        assert!(TransactionState::Failed.is_terminal());
        assert!(!TransactionState::Started.is_terminal());
        assert!(!TransactionState::Applied.is_terminal());
    }

    #[test]
    fn test_can_transition_to_valid_paths() {
        use TransactionInput as I;
        use TransactionState as S;

        assert!(S::Idle.can_transition_to(&S::Started, &I::Start));
        assert!(S::Started.can_transition_to(&S::Applied, &I::ValidateOk));
        assert!(S::Started.can_transition_to(&S::Failed, &I::ValidateFail));
        assert!(S::Applied.can_transition_to(&S::Committed, &I::Commit));
        assert!(S::Started.can_transition_to(&S::Cancelled, &I::Cancel));
        assert!(S::Applied.can_transition_to(&S::Cancelled, &I::Cancel));
    }

    #[test]
    fn test_can_transition_to_invalid_paths() {
        use TransactionInput as I;
        use TransactionState as S;

        assert!(!S::Idle.can_transition_to(&S::Committed, &I::Commit));
        assert!(!S::Applied.can_transition_to(&S::Started, &I::Start));
        assert!(!S::Failed.can_transition_to(&S::Committed, &I::Commit));
    }

    #[test]
    fn test_valid_transitions() {
        use TransactionInput as I;
        use TransactionState as S;

        assert_eq!(S::Idle.valid_transitions(&I::Start), vec![S::Started]);
        assert_eq!(
            S::Started.valid_transitions(&I::ValidateOk),
            vec![S::Applied]
        );
        assert_eq!(
            S::Started.valid_transitions(&I::ValidateFail),
            vec![S::Failed]
        );
        assert_eq!(S::Applied.valid_transitions(&I::Commit), vec![S::Committed]);

        // Cancel can be from Started or Applied
        assert_eq!(S::Started.valid_transitions(&I::Cancel), vec![S::Cancelled]);
        assert_eq!(S::Applied.valid_transitions(&I::Cancel), vec![S::Cancelled]);

        // No other transitions
        assert!(S::Committed.valid_transitions(&I::Cancel).is_empty());
    }

    #[test]
    fn test_transition_output_default() {
        // Output is default (no events)
        let out = TransactionState::Started
            .transition_output(&TransactionState::Applied, &TransactionInput::ValidateOk);
        assert!(out.events.is_empty());
    }

    #[test]
    fn test_txoutput_clone_drops_events() {
        #[derive(Debug)]
        struct E(uuid::Uuid);
        impl DomainEvent for E {
            fn aggregate_id(&self) -> uuid::Uuid {
                self.0
            }
            fn event_type(&self) -> &'static str {
                "E"
            }
        }
        let evt: Box<dyn DomainEvent> = Box::new(E(uuid::Uuid::new_v4()));
        let original = TxOutput { events: vec![evt] };
        let cloned = original.clone();
        assert!(cloned.events.is_empty());
    }
}
