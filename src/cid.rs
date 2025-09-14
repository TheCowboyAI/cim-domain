// Copyright 2025 Cowboy AI, LLC.

//! Content Identifiers (CIDs) as Domain Concepts
//!
//! CIDs provide content-addressed identity for domain objects.
//! They are a fundamental domain concept, not just a storage detail.
//! 
//! # Domain Model
//! 
//! In our FP domain model, CIDs serve as:
//! - Immutable identifiers for value objects
//! - Content verification for events
//! - Causality tracking through CID chains
//! - Integrity guarantees for distributed systems

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// Re-export the underlying CID type
pub use cid::Cid as CidImpl;
pub use cid::Error as CidError;
pub use multihash::Multihash;

/// Domain-specific CID wrapper with FP patterns
/// 
/// This wraps the raw CID with domain semantics and ensures
/// it follows our FP principles as an immutable value object.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomainCid {
    inner: CidImpl,
    /// Optional domain context
    domain: Option<String>,
    /// Content type hint
    content_type: ContentType,
}

/// Types of content that can have CIDs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ContentType {
    /// Domain event
    Event,
    /// Aggregate snapshot
    Aggregate,
    /// Value object
    ValueObject,
    /// Document or file
    Document,
    /// Raw binary data
    Raw,
}

impl DomainCid {
    /// Create a new domain CID from content
    pub fn from_content(bytes: &[u8], content_type: ContentType) -> Self {
        // Use Blake3 for hashing (fast and secure)
        let hash_bytes = blake3::hash(bytes);
        let mh = Multihash::wrap(0x1e, hash_bytes.as_bytes())
            .expect("Failed to create multihash");
        let cid = CidImpl::new_v1(0x55, mh); // RAW codec
        
        Self {
            inner: cid,
            domain: None,
            content_type,
        }
    }
    
    /// Create from existing CID
    pub fn from_cid(cid: CidImpl, content_type: ContentType) -> Self {
        Self {
            inner: cid,
            domain: None,
            content_type,
        }
    }
    
    /// Set domain context
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }
    
    /// Get the inner CID
    pub fn inner(&self) -> &CidImpl {
        &self.inner
    }
    
    /// Get content type
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }
    
    /// Get domain if set
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }
}

impl fmt::Display for DomainCid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl FromStr for DomainCid {
    type Err = CidError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cid = CidImpl::from_str(s)?;
        Ok(Self::from_cid(cid, ContentType::Raw))
    }
}

/// CID chain for event causality
/// 
/// Events form a chain through their CIDs, enabling:
/// - Causality tracking
/// - Integrity verification
/// - Event replay in correct order
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CidChain {
    /// Current CID
    pub current: DomainCid,
    /// Previous CID in chain (if any)
    pub previous: Option<DomainCid>,
    /// Correlation ID for related events
    pub correlation_id: String,
    /// Causation ID for event that caused this
    pub causation_id: String,
}

impl CidChain {
    /// Create a new chain entry
    pub fn new(
        content: &[u8],
        content_type: ContentType,
        correlation_id: String,
        causation_id: String,
        previous: Option<DomainCid>,
    ) -> Self {
        let current = DomainCid::from_content(content, content_type);
        
        Self {
            current,
            previous,
            correlation_id,
            causation_id,
        }
    }
    
    /// Check if this is the genesis (first) entry
    pub fn is_genesis(&self) -> bool {
        self.previous.is_none()
    }
    
    /// Verify chain integrity
    pub fn verify_chain(&self, previous_chain: &CidChain) -> bool {
        match &self.previous {
            Some(prev_cid) => prev_cid == &previous_chain.current,
            None => false,
        }
    }
}

/// Helper to generate CID from any serializable domain object
pub fn generate_cid<T: Serialize>(object: &T, content_type: ContentType) -> Result<DomainCid, String> {
    let bytes = serde_json::to_vec(object)
        .map_err(|e| format!("Failed to serialize object: {}", e))?;
    Ok(DomainCid::from_content(&bytes, content_type))
}

// ============================================================================
// FP PATTERNS
// ============================================================================

use crate::formal_domain::{DomainConcept, ValueObject};

/// CIDs are value objects - immutable and compared by value
impl DomainConcept for DomainCid {}
impl ValueObject for DomainCid {}

impl DomainConcept for CidChain {}
impl ValueObject for CidChain {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_domain_cid_creation() {
        let content = b"test content";
        let cid = DomainCid::from_content(content, ContentType::Event);
        
        assert_eq!(cid.content_type(), &ContentType::Event);
        assert!(cid.domain().is_none());
        
        // With domain
        let cid_with_domain = cid.clone().with_domain("test-domain".to_string());
        assert_eq!(cid_with_domain.domain(), Some("test-domain"));
    }
    
    #[test]
    fn test_cid_chain() {
        let content1 = b"event 1";
        let chain1 = CidChain::new(
            content1,
            ContentType::Event,
            "corr-123".to_string(),
            "cause-123".to_string(),
            None,
        );
        
        assert!(chain1.is_genesis());
        
        let content2 = b"event 2";
        let chain2 = CidChain::new(
            content2,
            ContentType::Event,
            "corr-123".to_string(),
            chain1.current.to_string(),
            Some(chain1.current.clone()),
        );
        
        assert!(!chain2.is_genesis());
        assert!(chain2.verify_chain(&chain1));
    }
    
    #[test]
    fn test_generate_cid() {
        #[derive(Serialize)]
        struct TestEvent {
            id: String,
            data: String,
        }
        
        let event = TestEvent {
            id: "test-123".to_string(),
            data: "test data".to_string(),
        };
        
        let cid = generate_cid(&event, ContentType::Event).unwrap();
        assert_eq!(cid.content_type(), &ContentType::Event);
    }
}