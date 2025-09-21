//! Domain-level data structures describing bucket and index semantics for content addressing.

use std::collections::VecDeque;

use crate::cid::DomainCid;
use crate::cqrs::EventId;

/// Identifies whether a bucket is rooted at an aggregate or a concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BucketRootKind {
    /// Bucket anchored on a domain aggregate.
    Aggregate,
    /// Bucket anchored on a concept (e.g. supporting artefacts).
    Concept,
}

/// Append-only record representing a single entry in a bucket DAG.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BucketEntry {
    /// CID persisted at this position in the bucket.
    pub cid: DomainCid,
    /// Previous tail (if any) when this entry was appended.
    pub previous_tail: Option<DomainCid>,
    /// Monotonic sequence index assigned within the bucket.
    pub sequence_index: u64,
}

impl BucketEntry {
    /// Create a new bucket entry with the supplied sequence number.
    pub fn new(cid: DomainCid, previous_tail: Option<DomainCid>, sequence_index: u64) -> Self {
        Self {
            cid,
            previous_tail,
            sequence_index,
        }
    }
}

/// In-memory helper that models the append-only sequence of a bucket.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BucketLog {
    /// Kind of root anchoring this bucket (aggregate or concept).
    pub root_kind: BucketRootKind,
    /// Identifier of the aggregate or concept root.
    pub root_id: String,
    /// Derived subject key (if known).
    pub subject_key: Option<String>,
    entries: VecDeque<BucketEntry>,
}

impl BucketLog {
    /// Create an empty bucket log for a given root and optional subject key.
    pub fn new(
        root_kind: BucketRootKind,
        root_id: impl Into<String>,
        subject_key: Option<String>,
    ) -> Self {
        Self {
            root_kind,
            root_id: root_id.into(),
            subject_key,
            entries: VecDeque::new(),
        }
    }

    /// Append a CID to the bucket and return the recorded entry.
    pub fn append(&mut self, cid: DomainCid) -> BucketEntry {
        let previous_tail = self.tail().cloned();
        let sequence_index = self.entries.len() as u64;
        let entry = BucketEntry::new(cid, previous_tail, sequence_index);
        self.entries.push_back(entry.clone());
        entry
    }

    /// Current tail CID, if any.
    pub fn tail(&self) -> Option<&DomainCid> {
        self.entries.back().map(|entry| &entry.cid)
    }

    /// Iterate over all entries in insertion order.
    pub fn entries(&self) -> impl Iterator<Item = &BucketEntry> {
        self.entries.iter()
    }

    /// Total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true when no entries have been appended yet.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Tracks a move event for a CID (from one bucket to another).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveHistoryEntry {
    /// Event that triggered the move.
    pub event_id: EventId,
    /// Source bucket identifier (if the CID already existed elsewhere).
    pub from_bucket: Option<String>,
    /// Destination bucket identifier.
    pub to_bucket: String,
}

/// Derived KV index entry capturing where a CID currently lives and how it relates to others.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CidIndexEntry {
    /// CID being indexed.
    pub cid: DomainCid,
    /// Bucket that currently contains this CID.
    pub bucket_id: String,
    /// Aggregate or concept root kind.
    pub root_kind: BucketRootKind,
    /// Aggregate or concept identifier.
    pub root_id: String,
    /// Optional subject algebra key.
    pub subject_key: Option<String>,
    /// Parent payload CID (only set for metadata/replacement records).
    pub payload_parent: Option<DomainCid>,
    /// Original payload (when this entry represents a replacement).
    pub replacement_for: Option<DomainCid>,
    /// Monotonic sequence index within the bucket.
    pub sequence_index: u64,
    /// Recorded move events.
    pub move_history: Vec<MoveHistoryEntry>,
}

impl CidIndexEntry {
    /// Construct a new index entry capturing the current bucket and relationships.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cid: DomainCid,
        bucket_id: impl Into<String>,
        root_kind: BucketRootKind,
        root_id: impl Into<String>,
        subject_key: Option<String>,
        payload_parent: Option<DomainCid>,
        replacement_for: Option<DomainCid>,
        sequence_index: u64,
    ) -> Self {
        Self {
            cid,
            bucket_id: bucket_id.into(),
            root_kind,
            root_id: root_id.into(),
            subject_key,
            payload_parent,
            replacement_for,
            sequence_index,
            move_history: Vec::new(),
        }
    }

    /// Record a move (from one bucket to another) originating from a given event.
    pub fn record_move(
        &mut self,
        event_id: EventId,
        from_bucket: Option<String>,
        to_bucket: impl Into<String>,
        new_sequence_index: u64,
    ) {
        let to_bucket = to_bucket.into();
        self.move_history.push(MoveHistoryEntry {
            event_id,
            from_bucket: from_bucket.clone(),
            to_bucket: to_bucket.clone(),
        });
        self.bucket_id = to_bucket;
        self.sequence_index = new_sequence_index;
    }
}

/// Convenience bundle returned when indexing a CID into a bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedCid {
    /// Entry appended to the bucket log.
    pub bucket_entry: BucketEntry,
    /// Corresponding CID index entry.
    pub index_entry: CidIndexEntry,
}

/// Append a root CID (aggregate/concept) and build its index entry in one step.
pub fn index_root_cid(
    bucket: &mut BucketLog,
    root_cid: DomainCid,
    bucket_id: impl Into<String>,
) -> IndexedCid {
    let entry = bucket.append(root_cid.clone());
    let index = CidIndexEntry::new(
        root_cid,
        bucket_id,
        bucket.root_kind,
        bucket.root_id.clone(),
        bucket.subject_key.clone(),
        None,
        None,
        entry.sequence_index,
    );
    IndexedCid {
        bucket_entry: entry,
        index_entry: index,
    }
}

/// Append a child CID (value object, metadata, replacement) linked to a parent.
pub fn index_child_cid(
    bucket: &mut BucketLog,
    parent_cid: &DomainCid,
    child_cid: DomainCid,
    bucket_id: impl Into<String>,
    subject_key: Option<String>,
    replacement_for: Option<DomainCid>,
) -> IndexedCid {
    let entry = bucket.append(child_cid.clone());
    let index = CidIndexEntry::new(
        child_cid,
        bucket_id,
        bucket.root_kind,
        bucket.root_id.clone(),
        subject_key,
        Some(parent_cid.clone()),
        replacement_for,
        entry.sequence_index,
    );
    IndexedCid {
        bucket_entry: entry,
        index_entry: index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cid::{ContentType, DomainCid};

    fn sample_cid(label: &str) -> DomainCid {
        DomainCid::from_content(label.as_bytes(), ContentType::Document)
    }

    #[test]
    fn index_root_records_bucket_state() {
        let mut log = BucketLog::new(BucketRootKind::Aggregate, "agg-123", None);
        let root_cid = sample_cid("root");

        let indexed = index_root_cid(&mut log, root_cid.clone(), "bucket:agg-123");

        assert_eq!(log.len(), 1);
        assert_eq!(indexed.bucket_entry.sequence_index, 0);
        assert_eq!(indexed.bucket_entry.cid, root_cid);
        assert!(indexed.index_entry.payload_parent.is_none());
        assert_eq!(indexed.index_entry.bucket_id, "bucket:agg-123");
        assert_eq!(indexed.index_entry.root_id, "agg-123");
    }

    #[test]
    fn index_child_links_to_parent() {
        let mut log = BucketLog::new(BucketRootKind::Aggregate, "agg-456", None);
        let root_cid = sample_cid("root");
        let value_cid = sample_cid("value");

        let _root = index_root_cid(&mut log, root_cid.clone(), "bucket:agg-456");
        let indexed = index_child_cid(
            &mut log,
            &root_cid,
            value_cid.clone(),
            "bucket:agg-456",
            Some("aggregate.root.value".into()),
            None,
        );

        assert_eq!(log.len(), 2);
        assert_eq!(indexed.index_entry.payload_parent, Some(root_cid));
        assert_eq!(
            indexed.index_entry.subject_key.as_deref(),
            Some("aggregate.root.value")
        );
        assert_eq!(indexed.index_entry.bucket_id, "bucket:agg-456");
        assert_eq!(indexed.bucket_entry.cid, value_cid);
    }
}
