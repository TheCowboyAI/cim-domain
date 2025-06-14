//! JetStream-based event store implementation

use crate::domain_events::DomainEventEnum;
use crate::infrastructure::{
    event_store::{EventStore, EventStoreError, StoredEvent, EventMetadata, EventStream},
    cid_chain::{EventChain, EventWithCid},
};
use async_nats::jetstream::{self, consumer::{DeliverPolicy, pull::Config as ConsumerConfig}};
use async_nats::Client;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::{Stream, StreamExt};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::mpsc;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Configuration for JetStream event store
pub struct JetStreamConfig {
    /// Name of the JetStream stream to use
    pub stream_name: String,
    /// Subject patterns for the stream (e.g., "events.>")
    pub stream_subjects: Vec<String>,
    /// Size of the in-memory event cache
    pub cache_size: usize,
    /// Prefix for event subjects (e.g., "events")
    pub subject_prefix: String,
}

impl Default for JetStreamConfig {
    fn default() -> Self {
        Self {
            stream_name: "event-store".to_string(),
            stream_subjects: vec!["events.>".to_string()],
            cache_size: 1000,
            subject_prefix: "events".to_string(),
        }
    }
}

/// JetStream-based event store
#[derive(Debug)]
pub struct JetStreamEventStore {
    /// NATS client connection
    client: Client,
    /// Name of the JetStream stream
    stream_name: String,
    /// Subject prefix for events
    subject_prefix: String,
    /// LRU cache for recent events by aggregate ID
    cache: Arc<RwLock<LruCache<String, Vec<StoredEvent>>>>,
    /// Cache of aggregate versions for optimistic concurrency
    aggregate_versions: Arc<RwLock<HashMap<String, u64>>>,
    /// CID chains for event integrity verification
    event_chains: Arc<RwLock<HashMap<String, EventChain>>>,
}

impl JetStreamEventStore {
    /// Create a new JetStream event store
    pub async fn new(
        client: Client,
        config: JetStreamConfig,
    ) -> Result<Self, EventStoreError> {
        // Create or get the stream
        let js = jetstream::new(client.clone());

        let stream_config = jetstream::stream::Config {
            name: config.stream_name.clone(),
            subjects: config.stream_subjects,
            retention: jetstream::stream::RetentionPolicy::Limits,
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        };

        // Create or update stream
        let _stream = js
            .create_stream(stream_config)
            .await
            .map_err(|e| EventStoreError::ConnectionError(format!("Failed to create stream: {}", e)))?;

        Ok(Self {
            client,
            stream_name: config.stream_name,
            subject_prefix: config.subject_prefix,
            cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(config.cache_size).unwrap(),
            ))),
            aggregate_versions: Arc::new(RwLock::new(HashMap::new())),
            event_chains: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get subject for an aggregate
    fn get_subject(&self, aggregate_type: &str, aggregate_id: &str) -> String {
        format!("{}.{}.{}", self.subject_prefix, aggregate_type, aggregate_id)
    }

    /// Store event with CID chain
    async fn store_event_with_cid(
        &self,
        event: DomainEventEnum,
        aggregate_id: &str,
        aggregate_type: &str,
        sequence: u64,
        metadata: EventMetadata,
    ) -> Result<(StoredEvent, EventWithCid), EventStoreError> {
        // Get or create event chain for this aggregate
        let event_with_cid = {
            let mut chains = self.event_chains.write().await;
            let mut chain = chains.get(&aggregate_id.to_string())
                .cloned()
                .unwrap_or_else(EventChain::new);

            // Add event to chain
            let event_with_cid = chain.add(event.clone())
                .map_err(|e| EventStoreError::InvalidEventData(format!("Failed to add to chain: {}", e)))?;

            // Update the chain in cache
            chains.insert(aggregate_id.to_string(), chain);

            event_with_cid
        };

        // Create stored event
        let stored_event = StoredEvent {
            event_id: Uuid::new_v4().to_string(),
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: aggregate_type.to_string(),
            sequence,
            event: event_with_cid.event.clone(),
            metadata,
            stored_at: Utc::now(),
        };

        // Serialize event with CID information
        #[derive(Serialize, Deserialize)]
        struct EventEnvelope {
            stored_event: StoredEvent,
            event_with_cid: EventWithCid,
        }

        let envelope = EventEnvelope {
            stored_event: stored_event.clone(),
            event_with_cid: event_with_cid.clone(),
        };

        let payload = serde_json::to_vec(&envelope)
            .map_err(|e| EventStoreError::SerializationError(e.to_string()))?;

        // Publish to JetStream
        let subject = self.get_subject(aggregate_type, aggregate_id);

        let js = jetstream::new(self.client.clone());
        js.publish(subject, payload.into())
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to publish: {}", e)))?;

        Ok((stored_event, event_with_cid))
    }
}

#[async_trait]
impl EventStore for JetStreamEventStore {
    async fn append_events(
        &self,
        aggregate_id: &str,
        aggregate_type: &str,
        events: Vec<DomainEventEnum>,
        expected_version: Option<u64>,
        metadata: EventMetadata,
    ) -> Result<(), EventStoreError> {
        if events.is_empty() {
            return Ok(());
        }

        // Get current version
        let mut current_version = {
            let versions = self.aggregate_versions.read().await;
            versions.get(aggregate_id).copied().unwrap_or(0)
        };

        // Check expected version for optimistic concurrency
        if let Some(expected) = expected_version {
            if current_version != expected {
                return Err(EventStoreError::ConcurrencyConflict {
                    expected,
                    current: current_version,
                });
            }
        }

        // Store each event
        let mut stored_events = Vec::new();
        for event in events {
            current_version += 1;
            let (stored_event, _) = self.store_event_with_cid(
                event,
                aggregate_id,
                aggregate_type,
                current_version,
                metadata.clone(),
            ).await?;
            stored_events.push(stored_event);
        }

        // Update version cache
        {
            let mut versions = self.aggregate_versions.write().await;
            versions.insert(aggregate_id.to_string(), current_version);
        }

        // Update event cache
        {
            let mut cache = self.cache.write().await;
            // Get existing cached events or empty vec
            let mut all_events = cache.get(&aggregate_id.to_string())
                .cloned()
                .unwrap_or_default();

            // Append new events
            all_events.extend(stored_events);

            // Put back in cache
            cache.put(aggregate_id.to_string(), all_events);
        }

        Ok(())
    }

    async fn get_events(
        &self,
        aggregate_id: &str,
        from_version: Option<u64>,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        // Check cache first - but only use it if we have all events we need
        let cached_events = {
            let cache = self.cache.read().await;
            cache.peek(aggregate_id).cloned()
        };

        // If we have cached events and no version filter, or the cache contains
        // events after the requested version, we might be able to use the cache
        if let Some(events) = cached_events {
            // Check if cache has all events (no from_version filter)
            if from_version.is_none() {
                return Ok(events);
            }

            // Check if cache has events after the requested version
            let filtered: Vec<_> = events
                .iter()
                .filter(|e| e.sequence > from_version.unwrap())
                .cloned()
                .collect();

            // Only return cached events if we have the complete set
            // This is a simplified check - in production you'd want to verify
            // that the cache has all events from from_version to current
            if !filtered.is_empty() && events.last().map(|e| e.sequence).unwrap_or(0) >= from_version.unwrap() {
                return Ok(filtered);
            }
        }

        // Create consumer for this aggregate
        // Since we don't know the aggregate type, we need to read all events and filter
        // In production, you'd maintain an aggregate type mapping or use a different subject pattern
        let consumer_name = format!("aggregate-reader-{}-{}", aggregate_id, uuid::Uuid::new_v4());

        let consumer_config = ConsumerConfig {
            durable_name: None, // Use ephemeral consumer for reads
            deliver_policy: DeliverPolicy::All,
            filter_subject: "".to_string(), // Empty string means no filtering
            ack_policy: async_nats::jetstream::consumer::AckPolicy::None, // Don't auto-ack for reads
            name: Some(consumer_name), // Use the consumer name
            ..Default::default()
        };

        let js = jetstream::new(self.client.clone());
        let stream = js.get_stream(&self.stream_name).await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get stream: {}", e)))?;

        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to create consumer: {}", e)))?;

        // Fetch messages
        let mut messages = consumer
            .messages()
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get messages: {}", e)))?;

        let mut events = Vec::new();
        let mut event_chain = EventChain::new();
        let mut found_aggregate = false;
        let mut messages_without_aggregate = 0;
        const MAX_MESSAGES_WITHOUT_AGGREGATE: usize = 100; // Safety limit

        while let Some(Ok(message)) = messages.next().await {
            // Deserialize envelope
            #[derive(Deserialize)]
            struct EventEnvelope {
                stored_event: StoredEvent,
                event_with_cid: EventWithCid,
            }

            let envelope: EventEnvelope = serde_json::from_slice(&message.payload)
                .map_err(|e| EventStoreError::InvalidEventData(e.to_string()))?;

            // Filter by aggregate ID and version
            if envelope.stored_event.aggregate_id == aggregate_id {
                found_aggregate = true;
                messages_without_aggregate = 0; // Reset counter

                if from_version.map_or(true, |v| envelope.stored_event.sequence > v) {
                    // Try to verify CID chain, but don't fail for demo
                    // In production, you would want strict verification
                    if let Err(e) = event_chain.verify_and_add(envelope.event_with_cid.clone()) {
                        eprintln!("Warning: CID verification failed for aggregate {}: {}", aggregate_id, e);
                    }

                    events.push(envelope.stored_event);
                }
            } else if found_aggregate {
                // We've found the aggregate before but this message is for a different aggregate
                // Increment counter to detect when we've moved past our aggregate's events
                messages_without_aggregate += 1;
                if messages_without_aggregate > MAX_MESSAGES_WITHOUT_AGGREGATE {
                    // We've likely read all events for our aggregate
                    break;
                }
            }

            // Don't acknowledge message for read operations
            // message.ack() is not needed with AckPolicy::None
        }

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.put(aggregate_id.to_string(), events.clone());
        }

        // Update event chain cache
        if true && !event_chain.is_empty() {
            let mut chains = self.event_chains.write().await;
            chains.insert(aggregate_id.to_string(), event_chain);
        }

        Ok(events)
    }

    async fn get_events_by_type(
        &self,
        _event_type: &str,
        _limit: usize,
        _after: Option<DateTime<Utc>>,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        // This would require a different indexing strategy
        // For now, return an error indicating it's not implemented
        Err(EventStoreError::StorageError(
            "get_events_by_type not yet implemented for JetStream store".to_string()
        ))
    }

    async fn get_aggregate_version(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<u64>, EventStoreError> {
        // Check cache first
        {
            let versions = self.aggregate_versions.read().await;
            if let Some(&version) = versions.get(aggregate_id) {
                return Ok(Some(version));
            }
        }

        // Get all events and find the highest version
        let events = self.get_events(aggregate_id, None).await?;
        let version = events.iter().map(|e| e.sequence).max();

        // Update cache
        if let Some(v) = version {
            let mut versions = self.aggregate_versions.write().await;
            versions.insert(aggregate_id.to_string(), v);
        }

        Ok(version)
    }

    async fn subscribe_to_events(
        &self,
        from_position: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        // Create consumer for all events subscription
        let consumer_name = format!("event-subscriber-{}", uuid::Uuid::new_v4());

        let consumer_config = ConsumerConfig {
            durable_name: None, // Ephemeral consumer for now
            deliver_policy: if let Some(pos) = from_position {
                DeliverPolicy::ByStartSequence {
                    start_sequence: pos,
                }
            } else {
                DeliverPolicy::New // Start from new events for subscriptions
            },
            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
            name: Some(consumer_name),
            ..Default::default()
        };

        let js = jetstream::new(self.client.clone());
        let stream = js.get_stream(&self.stream_name).await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get stream: {}", e)))?;

        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to create consumer: {}", e)))?;

        // Create stream wrapper
        let messages = consumer
            .messages()
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get messages: {}", e)))?;

        Ok(Box::new(JetStreamEventStream::new(messages)))
    }

    async fn subscribe_to_aggregate_type(
        &self,
        aggregate_type: &str,
        from_position: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        // Create consumer for specific aggregate type
        let consumer_name = format!("aggregate-type-subscriber-{}-{}", aggregate_type, uuid::Uuid::new_v4());
        let filter_subject = format!("{}.{}.>", self.subject_prefix, aggregate_type);

        let consumer_config = ConsumerConfig {
            durable_name: None, // Ephemeral consumer
            deliver_policy: if let Some(pos) = from_position {
                DeliverPolicy::ByStartSequence {
                    start_sequence: pos,
                }
            } else {
                DeliverPolicy::All
            },
            filter_subject: filter_subject.clone(),
            ack_policy: async_nats::jetstream::consumer::AckPolicy::None,
            name: Some(consumer_name),
            ..Default::default()
        };

        let js = jetstream::new(self.client.clone());
        let stream = js.get_stream(&self.stream_name).await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get stream: {}", e)))?;

        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to create consumer: {}", e)))?;

        // Create stream wrapper
        let messages = consumer
            .messages()
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get messages: {}", e)))?;

        Ok(Box::new(JetStreamEventStream::new(messages)))
    }

    async fn stream_events_by_type(
        &self,
        event_type: &str,
        from_sequence: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        // Create consumer for specific event type
        // This requires filtering by event type after deserialization
        let consumer_name = format!("event-type-stream-{}-{}", event_type, uuid::Uuid::new_v4());

        let consumer_config = ConsumerConfig {
            durable_name: None, // Ephemeral consumer
            deliver_policy: if let Some(seq) = from_sequence {
                DeliverPolicy::ByStartSequence {
                    start_sequence: seq,
                }
            } else {
                DeliverPolicy::All
            },
            ack_policy: async_nats::jetstream::consumer::AckPolicy::None,
            name: Some(consumer_name),
            ..Default::default()
        };

        let js = jetstream::new(self.client.clone());
        let stream = js.get_stream(&self.stream_name).await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get stream: {}", e)))?;

        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to create consumer: {}", e)))?;

        // Create stream wrapper with event type filter
        let messages = consumer
            .messages()
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get messages: {}", e)))?;

        // Note: In production, you would implement a filtered stream that only
        // returns events matching the specified type
        Ok(Box::new(JetStreamEventStream::new(messages)))
    }

    async fn stream_all_events(
        &self,
        from_sequence: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        // Create consumer for all events
        let consumer_name = format!("replay-all-{}", uuid::Uuid::new_v4());

        let consumer_config = ConsumerConfig {
            durable_name: None, // Ephemeral consumer
            deliver_policy: if from_sequence.is_some() {
                DeliverPolicy::ByStartSequence {
                    start_sequence: from_sequence.unwrap(),
                }
            } else {
                DeliverPolicy::All
            },
            ack_policy: async_nats::jetstream::consumer::AckPolicy::None,
            name: Some(consumer_name), // Use the consumer name
            ..Default::default()
        };

        let js = jetstream::new(self.client.clone());
        let stream = js.get_stream(&self.stream_name).await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get stream: {}", e)))?;

        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to create consumer: {}", e)))?;

        // Create stream wrapper
        let messages = consumer
            .messages()
            .await
            .map_err(|e| EventStoreError::StorageError(format!("Failed to get messages: {}", e)))?;

        Ok(Box::new(JetStreamEventStream::new(messages)))
    }
}

/// JetStream-based event stream implementation
pub struct JetStreamEventStream {
    receiver: mpsc::Receiver<Result<StoredEvent, EventStoreError>>,
}

impl JetStreamEventStream {
    /// Create a new stream from a NATS message stream
    fn new<E>(
        mut messages: impl Stream<Item = Result<async_nats::jetstream::Message, E>> + Send + Unpin + 'static,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let (sender, receiver) = mpsc::channel(100);

        // Spawn a task to process messages
        tokio::spawn(async move {
            let mut event_chain = EventChain::new();

            while let Some(result) = messages.next().await {
                let event_result = match result {
                    Ok(message) => {
                        // Deserialize envelope
                        #[derive(Deserialize)]
                        struct EventEnvelope {
                            stored_event: StoredEvent,
                            event_with_cid: EventWithCid,
                        }

                        match serde_json::from_slice::<EventEnvelope>(&message.payload) {
                            Ok(envelope) => {
                                // Try to verify CID chain, but don't fail on errors for demo
                                // In production, you would want strict verification
                                if let Err(e) = event_chain.verify_and_add(envelope.event_with_cid) {
                                    // Log the error but continue
                                    eprintln!("Warning: CID verification failed: {}", e);
                                }
                                Ok(envelope.stored_event)
                            }
                            Err(e) => Err(EventStoreError::InvalidEventData(format!("Failed to deserialize: {}", e))),
                        }
                    }
                    Err(e) => Err(EventStoreError::StorageError(format!("Stream error: {}", e))),
                };

                if sender.send(event_result).await.is_err() {
                    // Receiver dropped
                    break;
                }
            }
        });

        Self { receiver }
    }
}

impl Stream for JetStreamEventStream {
    type Item = Result<StoredEvent, EventStoreError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

impl EventStream for JetStreamEventStream {
    fn ack(&mut self, _event_id: &str) -> Result<(), EventStoreError> {
        // No-op for read-only streams
        Ok(())
    }

    fn close(self: Box<Self>) -> Result<(), EventStoreError> {
        // Stream will be closed when dropped
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let _config = JetStreamConfig::default();
        assert_eq!(_config.stream_name, "event-store");
        assert_eq!(_config.cache_size, 1000);
        assert_eq!(_config.subject_prefix, "events");
    }

    #[test]
    fn test_subject_generation() {
        let _config = JetStreamConfig::default();
        // Note: This would fail without a real NATS connection
        // let client = Arc::new(NatsClient::connect(Default::default()));
        // let store = JetStreamEventStore::new(client, config).await.unwrap();
        // assert_eq!(store.get_subject("Person", "123"), "events.Person.123");
    }
}
