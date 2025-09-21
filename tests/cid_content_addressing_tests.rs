use std::collections::BTreeMap;

use cim_domain::cid::CidChain;
use cim_domain::cid::{
    generate_cid, ContentType, DomainCid, DomainNode, DomainPayloadCodec, MetaVal,
};
use cim_domain::object_store::{BucketEntry, BucketLog, BucketRootKind, CidIndexEntry};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

#[test]
fn domain_cid_content_and_domain_metadata() {
    let payload = b"example-payload";
    let cid = DomainCid::from_content(payload, ContentType::Event).with_domain("demo".into());

    assert_eq!(cid.content_type(), &ContentType::Event);
    assert_eq!(cid.domain(), Some("demo"));

    // Display round-trips via FromStr, defaulting payload type to Raw (legacy behavior)
    let cid_string = cid.to_string();
    let parsed = cid_string.parse::<DomainCid>().expect("parse domain cid");
    assert_eq!(parsed.inner(), cid.inner());
    assert_eq!(parsed.content_type(), &ContentType::Raw);
}

#[test]
fn domain_node_from_payload_preserves_metadata_and_types() {
    let mut metadata = BTreeMap::new();
    metadata.insert("schema".into(), MetaVal::Str("event-v1".into()));
    metadata.insert("partition".into(), MetaVal::I64(5));

    let payload_bytes = serde_json::to_vec(&json!({"field": "value"})).unwrap();
    let (node, root_cid) = DomainNode::from_payload(
        &payload_bytes,
        DomainPayloadCodec::DagCbor,
        ContentType::Event,
        metadata.clone(),
    );

    assert_eq!(node.metadata, metadata);
    assert_eq!(node.payload_codec, DomainPayloadCodec::DagCbor);
    assert_eq!(node.payload_type, ContentType::Event);
    // Content addressing envelope is tagged as document + domain-node
    assert_eq!(root_cid.content_type(), &ContentType::Document);
    assert_eq!(root_cid.domain(), Some("domain-node"));
}

#[test]
fn domain_node_uses_codec_multicodec_mapping() {
    let payload_bytes = b"binary-payload";
    for (codec, expected_code) in [
        (DomainPayloadCodec::Raw, 0x55),
        (DomainPayloadCodec::DagCbor, 0x71),
        (DomainPayloadCodec::DagJson, 0x0129),
    ] {
        let (node, _) =
            DomainNode::from_payload(payload_bytes, codec, ContentType::Document, BTreeMap::new());
        assert_eq!(node.payload_codec.code(), expected_code);
    }
}

#[test]
fn cid_chain_detects_genesis_and_validates_links() {
    let correlation = "corr-1".to_string();
    let causation = "cause-1".to_string();

    let genesis = CidChain::new(
        b"event-1",
        ContentType::Event,
        correlation.clone(),
        causation.clone(),
        None,
    );
    assert!(genesis.is_genesis());

    let next = CidChain::new(
        b"event-2",
        ContentType::Event,
        correlation.clone(),
        causation,
        Some(genesis.current.clone()),
    );
    assert!(next.verify_chain(&genesis));

    let unrelated = CidChain::new(
        b"event-3",
        ContentType::Event,
        correlation,
        "cause-2".into(),
        None,
    );
    assert!(!next.verify_chain(&unrelated));
}

#[test]
fn cid_chain_requires_previous_for_non_genesis() {
    let entry = CidChain::new(
        b"event-no-prev",
        ContentType::Event,
        "corr".into(),
        "cause".into(),
        None,
    );
    // A non-genesis entry without previous cannot verify against another chain segment
    assert!(!entry.verify_chain(&entry));
}

#[derive(Serialize)]
struct ExamplePayload {
    value: i32,
}

#[test]
fn generate_cid_uses_serialized_payload() {
    let payload = ExamplePayload { value: 42 };
    let cid = generate_cid(&payload, ContentType::ValueObject).expect("generate cid");
    assert_eq!(cid.content_type(), &ContentType::ValueObject);
}

#[test]
fn meta_values_round_trip() {
    let mut metadata = BTreeMap::new();
    metadata.insert("text".into(), MetaVal::Str("hello".into()));
    metadata.insert("flag".into(), MetaVal::Bool(true));
    metadata.insert("count".into(), MetaVal::I64(123));
    metadata.insert("ratio".into(), MetaVal::F64(0.5));
    metadata.insert("ref".into(), MetaVal::Cid(Uuid::nil().to_string()));

    let payload_bytes = b"meta-test";
    let (node, _) = DomainNode::from_payload(
        payload_bytes,
        DomainPayloadCodec::Raw,
        ContentType::Document,
        metadata.clone(),
    );
    assert_eq!(node.metadata, metadata);
}

#[test]
fn bucket_entry_records_sequence() {
    let payload = DomainCid::from_content(b"payload", ContentType::Raw);
    let entry = BucketEntry::new(payload.clone(), None, 0);
    assert_eq!(entry.cid, payload);
    assert!(entry.previous_tail.is_none());
    assert_eq!(entry.sequence_index, 0);

    let metadata = DomainCid::from_content(b"meta", ContentType::Document);
    let entry2 = BucketEntry::new(metadata.clone(), Some(payload.clone()), 1);
    assert_eq!(entry2.previous_tail, Some(payload));
    assert_eq!(entry2.sequence_index, 1);
    assert_eq!(entry2.cid, metadata);
}

#[test]
fn cid_index_entry_tracks_moves() {
    let payload = DomainCid::from_content(b"payload", ContentType::Raw);
    let mut entry = CidIndexEntry::new(
        payload.clone(),
        "bucket-A",
        BucketRootKind::Aggregate,
        "order-123",
        Some("subject".into()),
        None,
        None,
        5,
    );
    assert_eq!(entry.bucket_id, "bucket-A");
    assert_eq!(entry.sequence_index, 5);
    assert!(entry.move_history.is_empty());

    let move_event = cim_domain::EventId::new();
    entry.record_move(move_event, Some("bucket-A".into()), "bucket-B", 10);

    assert_eq!(entry.bucket_id, "bucket-B");
    assert_eq!(entry.sequence_index, 10);
    assert_eq!(entry.move_history.len(), 1);
    let history = &entry.move_history[0];
    assert_eq!(history.event_id, move_event);
    assert_eq!(history.from_bucket.as_deref(), Some("bucket-A"));
    assert_eq!(history.to_bucket, "bucket-B");
}

#[test]
fn bucket_log_appends_in_sequence() {
    let mut log = BucketLog::new(
        BucketRootKind::Concept,
        "concept-42",
        Some("subject".into()),
    );
    assert!(log.is_empty());
    assert!(log.tail().is_none());

    let first = log.append(DomainCid::from_content(b"payload", ContentType::Document));
    assert_eq!(first.sequence_index, 0);
    assert!(first.previous_tail.is_none());
    assert_eq!(log.tail(), Some(&first.cid));

    let second = log.append(DomainCid::from_content(b"meta", ContentType::Document));
    assert_eq!(second.sequence_index, 1);
    assert_eq!(second.previous_tail, Some(first.cid.clone()));
    assert_eq!(log.tail(), Some(&second.cid));
    assert_eq!(log.len(), 2);

    let collected: Vec<_> = log.entries().map(|entry| entry.sequence_index).collect();
    assert_eq!(collected, vec![0, 1]);
}
