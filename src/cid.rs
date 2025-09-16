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

/// Domain-specific CID wrapper with FP patterns.
///
/// This represents a content-addressed identifier used throughout the domain.
/// It can refer to standalone payloads (e.g., events, aggregates, raw docs)
/// or serve as the root CID of a domain node that includes typed metadata.
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
        let mh = Multihash::wrap(0x1e, hash_bytes.as_bytes()).expect("Failed to create multihash");
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

// =========================================================================
// DOMAIN NODE (payload + typed metadata) — IPLD-friendly shape
// =========================================================================

/// Supported payload codecs for domain nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainPayloadCodec {
    /// IPLD raw block (rare; only for specific mimetypes)
    Raw,
    /// IPLD dag-cbor (primary structured data codec)
    DagCbor,
    /// IPLD dag-json (human-readable variant)
    DagJson,
}

impl DomainPayloadCodec {
    /// Multicodec code (best-effort mapping; reserved values in multicodec table)
    pub fn code(self) -> u64 {
        match self {
            DomainPayloadCodec::Raw => 0x55,
            DomainPayloadCodec::DagCbor => 0x71,
            // dag-json codepoint varies by table; keep distinct from cbor
            DomainPayloadCodec::DagJson => 0x0129,
        }
    }
}

/// Simple typed metadata value for domain nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum MetaVal {
    Str(String),
    I64(i64),
    F64(f64),
    Bool(bool),
    /// Reference to another CID (string form)
    Cid(String),
}

/// IPLD-friendly domain node: typed metadata + payload CID/codec
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainNode {
    /// Typed metadata (simple KV list)
    pub metadata: std::collections::BTreeMap<String, MetaVal>,
    /// Payload content-address (e.g., dag-cbor/json/raw)
    pub payload_cid: CidImpl,
    /// Payload codec hint
    pub payload_codec: DomainPayloadCodec,
}

impl DomainNode {
    /// Construct a new domain node from payload bytes, codec, and metadata
    pub fn from_payload(
        payload_bytes: &[u8],
        payload_codec: DomainPayloadCodec,
        metadata: std::collections::BTreeMap<String, MetaVal>,
    ) -> (Self, DomainCid) {
        let hash_bytes = blake3::hash(payload_bytes);
        let mh = Multihash::wrap(0x1e, hash_bytes.as_bytes()).expect("create multihash");
        let payload_cid = CidImpl::new_v1(payload_codec.code(), mh);
        let node = DomainNode { metadata, payload_cid, payload_codec };
        // Serialize node metadata deterministically to derive a root CID
        let node_bytes = serde_json::to_vec(&node).expect("serialize domain node");
        let node_hash = blake3::hash(&node_bytes);
        let node_mh = Multihash::wrap(0x1e, node_hash.as_bytes()).expect("wrap node blake3");
        let root_cid = CidImpl::new_v1(0x71, node_mh); // treat node envelope as dag-cbor-ish
        let dcid = DomainCid::from_cid(root_cid, ContentType::Document)
            .with_domain("domain-node".to_string());
        (node, dcid)
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
pub fn generate_cid<T: Serialize>(
    object: &T,
    content_type: ContentType,
) -> Result<DomainCid, String> {
    let bytes =
        serde_json::to_vec(object).map_err(|e| format!("Failed to serialize object: {}", e))?;
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
    use std::collections::BTreeMap;
    use std::str::FromStr as _;

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

    #[test]
    fn test_domain_node_cid_changes_with_metadata() {
        let payload = br#"{ "example": true }"#;
        let mut meta1 = BTreeMap::new();
        meta1.insert("schema".to_string(), MetaVal::Str("v1".to_string()));
        let (_node1, root1) = DomainNode::from_payload(payload, DomainPayloadCodec::DagJson, meta1);

        let mut meta2 = BTreeMap::new();
        meta2.insert("schema".to_string(), MetaVal::Str("v2".to_string()));
        let (_node2, root2) = DomainNode::from_payload(payload, DomainPayloadCodec::DagJson, meta2);

        assert_ne!(root1.inner().to_string(), root2.inner().to_string());
    }

    #[test]
    fn test_domain_node_payload_codec_annotation() {
        let payload = b"bytes";
        let meta = BTreeMap::new();
        let (node, _root) = DomainNode::from_payload(payload, DomainPayloadCodec::Raw, meta);
        assert!(matches!(node.payload_codec, DomainPayloadCodec::Raw));
    }

    #[test]
    fn test_domain_node_payload_cid_codec_matches_annotation() {
        let payload = br#"{ "example": true }"#;
        let meta = BTreeMap::new();

        // DagCbor
        let (node_cbor, root_cbor) = DomainNode::from_payload(payload, DomainPayloadCodec::DagCbor, meta.clone());
        assert_eq!(node_cbor.payload_cid.codec(), DomainPayloadCodec::DagCbor.code());
        // Root cid is envelope (we encode as dag-cbor)
        assert_eq!(root_cbor.inner().codec(), 0x71);

        // DagJson
        let (node_json, _root_json) = DomainNode::from_payload(payload, DomainPayloadCodec::DagJson, meta);
        assert_eq!(node_json.payload_cid.codec(), DomainPayloadCodec::DagJson.code());
    }

    #[test]
    fn test_domain_cid_from_content_uses_raw_and_roundtrips() {
        let content = b"hello";
        let dcid = DomainCid::from_content(content, ContentType::Document);
        // Underlying CID uses RAW codec 0x55 per from_content implementation
        assert_eq!(dcid.inner().codec(), 0x55);
        // Display/parse roundtrip
        let s = dcid.to_string();
        let parsed = DomainCid::from_str(&s).expect("parse cid");
        assert_eq!(parsed.inner(), dcid.inner());
    }

    #[test]
    fn test_domain_node_payload_cid_is_valid_ipld_cid() {
        let payload = br#"{ "ipld": "ok" }"#;
        let meta = BTreeMap::new();
        let (node, _root) = DomainNode::from_payload(payload, DomainPayloadCodec::DagJson, meta);
        // payload_is IPLD.Cid: ensure it parses with cid crate
        let cid_text = node.payload_cid.to_string();
        let parsed = CidImpl::from_str(&cid_text).expect("valid IPLD.Cid string");
        assert_eq!(parsed, node.payload_cid);
    }
}
