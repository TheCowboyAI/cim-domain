//! CID chain implementation for event integrity using cim-ipld

use crate::domain_events::DomainEventEnum;
use cim_ipld::{ChainedContent, ContentChain, TypedContent, ContentType, Error as IpldError, Cid};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when working with CIDs and chains
#[derive(Debug, Error)]
pub enum CidError {
    /// Error from the underlying IPLD library
    #[error("IPLD error: {0}")]
    IpldError(#[from] IpldError),

    /// Failed to serialize event data
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// CID is malformed or invalid
    #[error("Invalid CID: {0}")]
    InvalidCid(String),

    /// Chain integrity violation
    #[error("Chain error: {0}")]
    ChainError(String),
}

/// Errors that can occur during chain verification
#[derive(Debug, Error)]
pub enum ChainVerificationError {
    /// Chain is broken at a specific sequence
    #[error("Broken chain at sequence {sequence}: {reason}")]
    BrokenChain {
        /// The sequence number where the break occurred
        sequence: u64,
        /// Description of why the chain is broken
        reason: String,
    },

    /// CID is invalid at a specific sequence
    #[error("Invalid CID at sequence {sequence}: {reason}")]
    InvalidCid {
        /// The sequence number with the invalid CID
        sequence: u64,
        /// Description of why the CID is invalid
        reason: String
    },

    /// CID doesn't match expected value at a specific sequence
    #[error("CID mismatch at sequence {sequence}: {reason}")]
    CidMismatch {
        /// The sequence number where the mismatch occurred
        sequence: u64,
        /// Description of the mismatch
        reason: String,
    },
}

/// Event with CID for chain verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWithCid {
    /// The domain event
    pub event: DomainEventEnum,
    /// Content identifier for this event
    pub cid: Cid,
    /// CID of the previous event in the chain (None for first event)
    pub previous_cid: Option<Cid>,
}

/// Wrapper for domain events to implement TypedContent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWrapper {
    /// The wrapped domain event
    pub event: DomainEventEnum,
}

impl TypedContent for EventWrapper {
    const CODEC: u64 = 0x0200; // JSON codec
    const CONTENT_TYPE: ContentType = ContentType::Event;
}

/// Event chain for managing a sequence of domain events
#[derive(Debug, Clone)]
pub struct EventChain {
    chain: ContentChain<EventWrapper>,
}

impl Default for EventChain {
    fn default() -> Self {
        Self::new()
    }
}

impl EventChain {
    /// Create a new event chain
    pub fn new() -> Self {
        Self {
            chain: ContentChain::new(),
        }
    }

    /// Add an event to the chain
    pub fn add(&mut self, event: DomainEventEnum) -> Result<EventWithCid, CidError> {
        let wrapper = EventWrapper { event };
        let content = self.chain.append(wrapper)?;
        Ok(EventWithCid {
            event: content.content.event.clone(),
            cid: content.cid.parse().map_err(|e| CidError::InvalidCid(format!("{e:?}")))?,
            previous_cid: content.previous_cid.clone().and_then(|s| s.parse().ok()),
        })
    }

    /// Verify and add an event with CID to the chain
    pub fn verify_and_add(&mut self, event_with_cid: EventWithCid) -> Result<(), CidError> {
        // Verify the chain
        if let Some(head) = self.chain.head() {
            if event_with_cid.previous_cid != Some(head.cid.parse().map_err(|e| CidError::InvalidCid(format!("{e:?}")))?) {
                return Err(CidError::ChainError(format!(
                    "Previous CID mismatch: expected {:?}, got {:?}",
                    head.cid,
                    event_with_cid.previous_cid
                )));
            }
        } else if event_with_cid.previous_cid.is_some() {
            return Err(CidError::ChainError(
                "Chain is empty but event has previous CID".to_string()
            ));
        }

        // Create wrapper from the event and add to chain
        let wrapper = EventWrapper { event: event_with_cid.event };
        let chained = self.chain.append(wrapper)?;

        // Verify the CID matches
        let expected_cid = event_with_cid.cid;
        let actual_cid = chained.cid.parse().map_err(|e| CidError::InvalidCid(format!("{e:?}")))?;
        if expected_cid != actual_cid {
            return Err(CidError::ChainError(format!(
                "CID mismatch: expected {expected_cid:?}, got {actual_cid:?}"
            )));
        }

        Ok(())
    }

    /// Get the head of the chain
    pub fn head(&self) -> Option<&ChainedContent<EventWrapper>> {
        self.chain.head()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }
}

/// Create an event with CID using cim-ipld
pub fn create_event_with_cid(
    event: DomainEventEnum,
    previous: Option<&EventWithCid>,
) -> Result<EventWithCid, CidError> {
    let wrapper = EventWrapper { event: event.clone() };
    let previous_chained = previous.map(|p| ChainedContent {
        content: EventWrapper { event: p.event.clone() },
        cid: p.cid.to_string(),
        previous_cid: p.previous_cid.map(|c| c.to_string()),
        sequence: 0, // Not used for this purpose
        timestamp: std::time::SystemTime::now(),
    });

    let chained = ChainedContent::new(wrapper, previous_chained.as_ref())?;

    Ok(EventWithCid {
        event,
        cid: chained.cid.parse().map_err(|e| CidError::InvalidCid(format!("{e:?}")))?,
        previous_cid: chained.previous_cid.and_then(|s| s.parse().ok()),
    })
}

/// Calculate CID for an event (compatibility function)
pub fn calculate_event_cid(
    event: &DomainEventEnum,
    previous_cid: Option<cim_ipld::Cid>,
    sequence: u64,
) -> Result<cim_ipld::Cid, CidError> {
    // Create a temporary wrapper
    let wrapper = EventWrapper { event: event.clone() };

    // Create a temporary chained content to calculate CID
    let temp_previous = previous_cid.map(|cid| {
        ChainedContent {
            content: EventWrapper { event: event.clone() }, // Dummy content
            cid: cid.to_string(),
            previous_cid: None,
            sequence: sequence.saturating_sub(1),
            timestamp: std::time::SystemTime::now(),
        }
    });

    let chained = ChainedContent::new(wrapper, temp_previous.as_ref())?;

    // Parse the CID string back to Cid
    cim_ipld::Cid::try_from(chained.cid.as_str())
        .map_err(|e| CidError::InvalidCid(e.to_string()))
}

/// Verify a chain of events
pub fn verify_event_chain(events: &[EventWithCid]) -> Result<(), CidError> {
    let mut previous: Option<&EventWithCid> = None;

    for (i, event) in events.iter().enumerate() {
        // Verify sequence (if we're tracking it separately)
        // For now, just verify CID chain

        // Verify CID chain
        match (previous, &event.previous_cid) {
            (None, None) => {
                // First event, no previous CID
            }
            (Some(prev), Some(prev_cid)) => {
                if prev.cid != *prev_cid {
                    return Err(CidError::ChainError(format!(
                        "CID chain broken at index {}: expected {:?}, got {:?}",
                        i, prev.cid, prev_cid
                    )));
                }
            }
            _ => {
                return Err(CidError::ChainError(format!(
                    "CID chain mismatch at index {i}"
                )));
            }
        }

        previous = Some(event);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_events::WorkflowStarted;
    use crate::identifiers::{WorkflowId, GraphId};

    fn create_test_event() -> DomainEventEnum {
        DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id: WorkflowId::new(),
            definition_id: GraphId::new(),
            initial_state: "Start".to_string(),
            started_at: chrono::Utc::now(),
        })
    }

    #[test]
    fn test_calculate_event_cid() {
        let event = create_test_event();
        let cid = calculate_event_cid(&event, None, 1).unwrap();

        // CID should be valid
        assert_eq!(cid.version(), cid::Version::V1);
    }

    #[test]
    fn test_event_chain_verification() {

        // Create a chain of events
        let event1 = create_event_with_cid(
            create_test_event(),
            None,
        ).unwrap();

        let event2 = create_event_with_cid(
            create_test_event(),
            Some(&event1),
        ).unwrap();

        let event3 = create_event_with_cid(
            create_test_event(),
            Some(&event2),
        ).unwrap();

        let chain = vec![event1, event2, event3];

        // Chain should be valid
        assert!(verify_event_chain(&chain).is_ok());
    }

    #[test]
    fn test_broken_chain_detection() {

        // Create events with broken chain
        let event1 = create_event_with_cid(
            create_test_event(),
            None,
        ).unwrap();

        // Create a fake event with wrong previous CID
        let mut event3 = create_event_with_cid(
            create_test_event(),
            None,
        ).unwrap();
        // This event should have event1 as previous, but it doesn't
        event3.previous_cid = Some(cim_ipld::Cid::try_from("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi").unwrap());

        let chain = vec![event1, event3];

        // Should detect broken chain
        let result = verify_event_chain(&chain);
        assert!(result.is_err());
    }
}
