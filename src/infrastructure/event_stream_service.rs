//! Event stream service implementation

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use futures::stream::StreamExt;

use crate::infrastructure::{
    EventStore, StoredEvent,
    EventStream, EventStreamId, EventStreamOperations, EventStreamError,
    EventQuery, EventFilter, EventOrdering,
    StreamTransformation, StreamComposition, GroupingCriteria,
};

/// Service for managing event streams
pub struct EventStreamService {
    event_store: Arc<dyn EventStore>,
    saved_streams: Arc<RwLock<HashMap<EventStreamId, EventStream>>>,
}

impl EventStreamService {
    /// Create a new event stream service
    pub fn new(event_store: Arc<dyn EventStore>) -> Self {
        Self {
            event_store,
            saved_streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an event matches the given filters
    fn matches_filters(&self, event: &StoredEvent, filters: &[EventFilter]) -> bool {
        filters.iter().all(|filter| match filter {
            EventFilter::EventType(event_type) => event.event_type() == event_type,
            EventFilter::EventTypes(types) => types.iter().any(|t| t == event.event_type()),
            EventFilter::AggregateId(agg_id) => &event.aggregate_id == agg_id,
            EventFilter::AggregateType(agg_type) => &event.aggregate_type == agg_type,
            EventFilter::AggregateTypes(types) => types.contains(&event.aggregate_type),
            EventFilter::CorrelationId(corr_id) => {
                event.correlation_id() == Some(corr_id)
            }
            EventFilter::MetadataValue { key, value } => {
                event.metadata.get(key).map(|v| v == value).unwrap_or(false)
            }
        })
    }

    /// Apply transformation to events
    fn apply_transformation(
        &self,
        events: Vec<StoredEvent>,
        transformation: &StreamTransformation,
    ) -> Result<Vec<StoredEvent>, EventStreamError> {
        match transformation {
            StreamTransformation::Filter(filter) => {
                Ok(events.into_iter()
                    .filter(|e| self.matches_filters(e, &[filter.clone()]))
                    .collect())
            }

            StreamTransformation::GroupBy(criteria) => {
                // For now, just return events grouped (not actually transforming structure)
                // In a full implementation, this would return grouped structure
                match criteria {
                    GroupingCriteria::AggregateType => {
                        let mut sorted = events;
                        sorted.sort_by_key(|e| e.aggregate_type.clone());
                        Ok(sorted)
                    }
                    GroupingCriteria::CorrelationId => {
                        let mut sorted = events;
                        sorted.sort_by_key(|e| e.correlation_id().map(|s| s.to_string()).unwrap_or_default());
                        Ok(sorted)
                    }
                    GroupingCriteria::EventType => {
                        let mut sorted = events;
                        sorted.sort_by_key(|e| e.event_type().to_string());
                        Ok(sorted)
                    }
                    GroupingCriteria::TimeWindow(_duration) => {
                        // Would implement time-based grouping
                        Ok(events)
                    }
                }
            }

            StreamTransformation::Window(_spec) => {
                // Windowing would be implemented here
                Ok(events)
            }
        }
    }

    /// Compose multiple event streams
    fn compose_event_lists(
        &self,
        streams: &[Vec<StoredEvent>],
        composition: &StreamComposition,
    ) -> Result<Vec<StoredEvent>, EventStreamError> {
        if streams.is_empty() {
            return Ok(Vec::new());
        }

        match composition {
            StreamComposition::Union => {
                // Union all events, removing duplicates by event_id
                let mut seen = HashSet::new();
                let mut result = Vec::new();

                for stream in streams {
                    for event in stream {
                        if seen.insert(event.event_id.clone()) {
                            result.push(event.clone());
                        }
                    }
                }

                Ok(result)
            }

            StreamComposition::Intersection => {
                // Only events that appear in all streams
                let first = &streams[0];
                let mut result = Vec::new();

                for event in first {
                    if streams[1..].iter().all(|stream| {
                        stream.iter().any(|e| e.event_id == event.event_id)
                    }) {
                        result.push(event.clone());
                    }
                }

                Ok(result)
            }

            StreamComposition::Difference => {
                // Events in first stream but not in others
                let first = &streams[0];
                let other_ids: HashSet<String> = streams[1..]
                    .iter()
                    .flat_map(|s| s.iter().map(|e| e.event_id.clone()))
                    .collect();

                let result = first
                    .iter()
                    .filter(|e| !other_ids.contains(&e.event_id))
                    .cloned()
                    .collect();

                Ok(result)
            }

            StreamComposition::Merge(resolution) => {
                // Merge with conflict resolution
                let mut event_map = HashMap::new();

                for stream in streams {
                    for event in stream {
                        match resolution {
                            crate::infrastructure::event_stream::ConflictResolution::KeepFirst => {
                                event_map.entry(event.event_id.clone())
                                    .or_insert_with(|| event.clone());
                            }
                            crate::infrastructure::event_stream::ConflictResolution::KeepLast => {
                                event_map.insert(event.event_id.clone(), event.clone());
                            }
                            crate::infrastructure::event_stream::ConflictResolution::KeepAll => {
                                // Would need different structure to keep all versions
                                event_map.insert(event.event_id.clone(), event.clone());
                            }
                            crate::infrastructure::event_stream::ConflictResolution::Custom(_) => {
                                return Err(EventStreamError::InvalidOperation(
                                    "Custom conflict resolution not implemented".to_string()
                                ));
                            }
                        }
                    }
                }

                Ok(event_map.into_values().collect())
            }
        }
    }
}

#[async_trait]
impl EventStreamOperations for EventStreamService {
    async fn create_stream(
        &self,
        name: String,
        description: String,
        query: EventQuery,
    ) -> Result<EventStream, EventStreamError> {
        let events = match &query {
            EventQuery::ByCorrelationId { correlation_id, .. } => {
                // Get all events and filter by correlation ID
                let mut all_events = self.event_store
                    .stream_all_events(None)
                    .await
                    .map_err(|e| EventStreamError::EventStoreError(e.to_string()))?;

                let mut events = Vec::new();
                while let Some(result) = all_events.next().await {
                    match result {
                        Ok(event) => {
                            if event.correlation_id() == Some(correlation_id) {
                                events.push(event);
                            }
                        }
                        Err(e) => return Err(EventStreamError::EventStoreError(e.to_string())),
                    }
                }
                events
            }
            EventQuery::ByTimeRange { start, end } => {
                // Get all events and filter by time range
                let mut all_events = self.event_store
                    .stream_all_events(None)
                    .await
                    .map_err(|e| EventStreamError::EventStoreError(e.to_string()))?;

                let mut events = Vec::new();
                while let Some(result) = all_events.next().await {
                    match result {
                        Ok(event) => {
                            let timestamp = event.timestamp();
                            if timestamp >= *start && timestamp <= *end {
                                events.push(event);
                            }
                        }
                        Err(e) => return Err(EventStreamError::EventStoreError(e.to_string())),
                    }
                }
                events
            }
            EventQuery::ByAggregateType { aggregate_type } => {
                // Get all events and filter by aggregate type
                let mut all_events = self.event_store
                    .stream_all_events(None)
                    .await
                    .map_err(|e| EventStreamError::EventStoreError(e.to_string()))?;

                let mut events = Vec::new();
                while let Some(result) = all_events.next().await {
                    match result {
                        Ok(event) => {
                            if &event.aggregate_type == aggregate_type {
                                events.push(event);
                            }
                        }
                        Err(e) => return Err(EventStreamError::EventStoreError(e.to_string())),
                    }
                }
                events
            }
            EventQuery::Complex { filters, ordering, limit } => {
                // Get all events and apply filters
                let mut all_events = self.event_store
                    .stream_all_events(None)
                    .await
                    .map_err(|e| EventStreamError::EventStoreError(e.to_string()))?;

                let mut events = Vec::new();
                while let Some(result) = all_events.next().await {
                    match result {
                        Ok(event) => {
                            if self.matches_filters(&event, filters) {
                                events.push(event);
                            }
                        }
                        Err(e) => return Err(EventStreamError::EventStoreError(e.to_string())),
                    }
                }

                // Apply ordering
                match ordering {
                    EventOrdering::Temporal => events.sort_by_key(|e| e.stored_at),
                    EventOrdering::Causal => {
                        // Sort by causation - events without causation first, then by causation chain
                        events.sort_by(|a, b| {
                            match (a.causation_id(), b.causation_id()) {
                                (None, None) => a.stored_at.cmp(&b.stored_at),
                                (None, Some(_)) => std::cmp::Ordering::Less,
                                (Some(_), None) => std::cmp::Ordering::Greater,
                                (Some(a_caus), Some(b_caus)) => {
                                    a_caus.cmp(&b_caus).then_with(|| a.stored_at.cmp(&b.stored_at))
                                }
                            }
                        });
                    }
                    EventOrdering::AggregateSequence => {
                        events.sort_by(|a, b| {
                            a.aggregate_id.cmp(&b.aggregate_id)
                                .then_with(|| a.sequence.cmp(&b.sequence))
                        });
                    }
                }

                // Apply limit
                if let Some(limit) = limit {
                    events.truncate(*limit);
                }

                events
            }
            EventQuery::ByWorkflowExecution { instance_id, correlation_ids } => {
                // Get events for workflow instance
                let mut events = self.event_store
                    .get_events(instance_id, None)
                    .await
                    .map_err(|e| EventStreamError::EventStoreError(e.to_string()))?;

                // Also get events by correlation IDs
                let mut all_events = self.event_store
                    .stream_all_events(None)
                    .await
                    .map_err(|e| EventStreamError::EventStoreError(e.to_string()))?;

                while let Some(result) = all_events.next().await {
                    match result {
                        Ok(event) => {
                            if let Some(corr_id) = event.correlation_id() {
                                if correlation_ids.contains(corr_id) {
                                    events.push(event);
                                }
                            }
                        }
                        Err(e) => return Err(EventStreamError::EventStoreError(e.to_string())),
                    }
                }

                // Remove duplicates
                let mut seen = HashSet::new();
                events.retain(|e| seen.insert(e.event_id.clone()));

                events
            }
        };

        Ok(EventStream::new(name, description, query, events))
    }

    async fn transform_stream(
        &self,
        stream: &EventStream,
        transformation: StreamTransformation,
    ) -> Result<EventStream, EventStreamError> {
        let transformed_events = self.apply_transformation(
            stream.events.clone(),
            &transformation,
        )?;

        Ok(EventStream::new(
            format!("{} (transformed)", stream.name),
            stream.description.clone(),
            stream.query.clone(),
            transformed_events,
        ))
    }

    async fn compose_streams(
        &self,
        streams: Vec<EventStream>,
        composition: StreamComposition,
    ) -> Result<EventStream, EventStreamError> {
        if streams.is_empty() {
            return Err(EventStreamError::InvalidOperation(
                "Cannot compose empty stream list".to_string()
            ));
        }

        let event_lists: Vec<Vec<StoredEvent>> = streams
            .iter()
            .map(|s| s.events.clone())
            .collect();

        let composed_events = self.compose_event_lists(&event_lists, &composition)?;

        let name = format!(
            "Composed: {}",
            streams.iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(" + ")
        );

        Ok(EventStream::new(
            name,
            "Composed event stream".to_string(),
            streams[0].query.clone(), // Use first stream's query as base
            composed_events,
        ))
    }

    async fn save_stream(
        &self,
        stream: &EventStream,
    ) -> Result<(), EventStreamError> {
        let mut saved = self.saved_streams.write().await;
        saved.insert(stream.id.clone(), stream.clone());
        Ok(())
    }

    async fn load_stream(
        &self,
        stream_id: &EventStreamId,
    ) -> Result<EventStream, EventStreamError> {
        let saved = self.saved_streams.read().await;
        saved.get(stream_id)
            .cloned()
            .ok_or_else(|| EventStreamError::StreamNotFound(stream_id.clone()))
    }

    async fn list_streams(
        &self,
    ) -> Result<Vec<EventStream>, EventStreamError> {
        let saved = self.saved_streams.read().await;
        Ok(saved.values().cloned().collect())
    }
}
