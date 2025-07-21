// Copyright 2025 Cowboy AI, LLC.

//! Metrics collection for persistence operations

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Metrics for persistence operations
#[derive(Debug, Clone)]
pub struct PersistenceMetrics {
    counters: Arc<RwLock<HashMap<String, u64>>>,
    durations: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    errors: Arc<RwLock<HashMap<String, u64>>>,
}

impl Default for PersistenceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistenceMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            durations: Arc::new(RwLock::new(HashMap::new())),
            errors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Increment a counter
    pub async fn increment(&self, name: &str) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Record an error
    pub async fn record_error(&self, operation: &str) {
        let mut errors = self.errors.write().await;
        *errors.entry(operation.to_string()).or_insert(0) += 1;
    }

    /// Record operation duration
    pub async fn record_duration(&self, operation: &str, duration: Duration) {
        let mut durations = self.durations.write().await;
        durations
            .entry(operation.to_string())
            .or_default()
            .push(duration);

        // Keep only last 1000 measurements per operation
        if let Some(vec) = durations.get_mut(operation) {
            if vec.len() > 1000 {
                vec.drain(0..vec.len() - 1000);
            }
        }
    }

    /// Get counter value
    pub async fn get_counter(&self, name: &str) -> u64 {
        let counters = self.counters.read().await;
        counters.get(name).copied().unwrap_or(0)
    }

    /// Get error count
    pub async fn get_error_count(&self, operation: &str) -> u64 {
        let errors = self.errors.read().await;
        errors.get(operation).copied().unwrap_or(0)
    }

    /// Get average duration for an operation
    pub async fn get_avg_duration(&self, operation: &str) -> Option<Duration> {
        let durations = self.durations.read().await;
        if let Some(vec) = durations.get(operation) {
            if vec.is_empty() {
                return None;
            }
            let sum: Duration = vec.iter().sum();
            Some(sum / vec.len() as u32)
        } else {
            None
        }
    }

    /// Get percentile duration
    pub async fn get_percentile_duration(
        &self,
        operation: &str,
        percentile: f64,
    ) -> Option<Duration> {
        let durations = self.durations.read().await;
        if let Some(vec) = durations.get(operation) {
            if vec.is_empty() {
                return None;
            }
            let mut sorted = vec.clone();
            sorted.sort();
            let index = ((sorted.len() as f64 - 1.0) * percentile / 100.0) as usize;
            Some(sorted[index])
        } else {
            None
        }
    }

    /// Get all metrics as a summary
    pub async fn summary(&self) -> MetricsSummary {
        let counters = self.counters.read().await.clone();
        let errors = self.errors.read().await.clone();

        let mut duration_stats = HashMap::new();
        let durations = self.durations.read().await;

        for (op, vec) in durations.iter() {
            if !vec.is_empty() {
                let sum: Duration = vec.iter().sum();
                let avg = sum / vec.len() as u32;

                let mut sorted = vec.clone();
                sorted.sort();
                let p50_idx = ((sorted.len() as f64 - 1.0) * 0.5) as usize;
                let p95_idx = ((sorted.len() as f64 - 1.0) * 0.95) as usize;
                let p99_idx = ((sorted.len() as f64 - 1.0) * 0.99) as usize;

                duration_stats.insert(
                    op.clone(),
                    DurationStats {
                        count: vec.len(),
                        avg,
                        p50: sorted[p50_idx],
                        p95: sorted[p95_idx],
                        p99: sorted[p99_idx],
                        min: *sorted.first().unwrap(),
                        max: *sorted.last().unwrap(),
                    },
                );
            }
        }

        MetricsSummary {
            counters,
            errors,
            durations: duration_stats,
        }
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.counters.write().await.clear();
        self.durations.write().await.clear();
        self.errors.write().await.clear();
    }
}

/// Summary of all metrics
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    /// Counter values
    pub counters: HashMap<String, u64>,
    /// Error counts
    pub errors: HashMap<String, u64>,
    /// Duration statistics
    pub durations: HashMap<String, DurationStats>,
}

/// Duration statistics for an operation
#[derive(Debug, Clone)]
pub struct DurationStats {
    /// Number of measurements
    pub count: usize,
    /// Average duration
    pub avg: Duration,
    /// 50th percentile
    pub p50: Duration,
    /// 95th percentile
    pub p95: Duration,
    /// 99th percentile
    pub p99: Duration,
    /// Minimum duration
    pub min: Duration,
    /// Maximum duration
    pub max: Duration,
}

/// Timer for measuring operation duration
pub struct MetricsTimer<'a> {
    metrics: &'a PersistenceMetrics,
    operation: String,
    start: Instant,
}

impl<'a> MetricsTimer<'a> {
    /// Create a new timer
    pub fn new(metrics: &'a PersistenceMetrics, operation: &str) -> Self {
        Self {
            metrics,
            operation: operation.to_string(),
            start: Instant::now(),
        }
    }

    /// Record the duration and increment counter
    pub async fn record(self) {
        let duration = self.start.elapsed();
        self.metrics
            .record_duration(&self.operation, duration)
            .await;
        self.metrics
            .increment(&format!("{}.count", self.operation))
            .await;
    }

    /// Record as error
    pub async fn record_error(self) {
        let duration = self.start.elapsed();
        self.metrics
            .record_duration(&self.operation, duration)
            .await;
        self.metrics.record_error(&self.operation).await;
    }
}

/// Trait for instrumenting repositories with metrics
#[async_trait::async_trait]
pub trait MetricsInstrumented {
    /// Get metrics collector
    fn metrics(&self) -> &PersistenceMetrics;

    /// Create a timer for an operation
    fn timer(&self, operation: &str) -> MetricsTimer<'_> {
        MetricsTimer::new(self.metrics(), operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_counters() {
        let metrics = PersistenceMetrics::new();

        metrics.increment("test.counter").await;
        metrics.increment("test.counter").await;

        assert_eq!(metrics.get_counter("test.counter").await, 2);
        assert_eq!(metrics.get_counter("nonexistent").await, 0);
    }

    #[tokio::test]
    async fn test_errors() {
        let metrics = PersistenceMetrics::new();

        metrics.record_error("save").await;
        metrics.record_error("save").await;
        metrics.record_error("load").await;

        assert_eq!(metrics.get_error_count("save").await, 2);
        assert_eq!(metrics.get_error_count("load").await, 1);
    }

    #[tokio::test]
    async fn test_durations() {
        let metrics = PersistenceMetrics::new();

        metrics
            .record_duration("op1", Duration::from_millis(10))
            .await;
        metrics
            .record_duration("op1", Duration::from_millis(20))
            .await;
        metrics
            .record_duration("op1", Duration::from_millis(30))
            .await;

        let avg = metrics.get_avg_duration("op1").await.unwrap();
        assert_eq!(avg, Duration::from_millis(20));

        let p50 = metrics.get_percentile_duration("op1", 50.0).await.unwrap();
        assert_eq!(p50, Duration::from_millis(20));
    }

    #[tokio::test]
    async fn test_timer() {
        let metrics = PersistenceMetrics::new();

        {
            let timer = MetricsTimer::new(&metrics, "test.operation");
            tokio::time::sleep(Duration::from_millis(10)).await;
            timer.record().await;
        }

        assert_eq!(metrics.get_counter("test.operation.count").await, 1);
        assert!(
            metrics.get_avg_duration("test.operation").await.unwrap() >= Duration::from_millis(10)
        );
    }

    #[tokio::test]
    async fn test_summary() {
        let metrics = PersistenceMetrics::new();

        metrics.increment("saves").await;
        metrics.record_error("load").await;
        metrics
            .record_duration("save", Duration::from_millis(5))
            .await;
        metrics
            .record_duration("save", Duration::from_millis(15))
            .await;

        let summary = metrics.summary().await;

        assert_eq!(summary.counters.get("saves"), Some(&1));
        assert_eq!(summary.errors.get("load"), Some(&1));
        assert_eq!(summary.durations.get("save").unwrap().count, 2);
        assert_eq!(
            summary.durations.get("save").unwrap().avg,
            Duration::from_millis(10)
        );
    }

    // ===== CONCURRENT TESTING MODULE =====

    /// Tests for concurrent metrics updates and race conditions
    mod concurrent_tests {
        use super::*;
        use std::sync::atomic::{AtomicU64, Ordering};
        use tokio::task::JoinHandle;

        #[tokio::test]
        async fn test_concurrent_counter_increments() {
            let metrics = Arc::new(PersistenceMetrics::new());
            let expected_total = 1000u64;
            let num_tasks = 20;
            let increments_per_task = expected_total / num_tasks;

            let mut handles: Vec<JoinHandle<()>> = vec![];

            for task_id in 0..num_tasks {
                let metrics_clone = metrics.clone();
                handles.push(tokio::spawn(async move {
                    for i in 0..increments_per_task {
                        // Test multiple counter types
                        metrics_clone.increment("counter.shared").await;
                        metrics_clone
                            .increment(&format!("counter.task_{}", task_id))
                            .await;

                        // Also test interspersed operations
                        if i.is_multiple_of(10) {
                            metrics_clone.increment("counter.periodic").await;
                        }
                    }
                }));
            }

            // Wait for all tasks
            for handle in handles {
                handle.await.unwrap();
            }

            // Verify counters
            assert_eq!(metrics.get_counter("counter.shared").await, expected_total);
            assert_eq!(
                metrics.get_counter("counter.periodic").await,
                expected_total / 10
            );

            // Each task counter should have correct count
            for task_id in 0..num_tasks {
                assert_eq!(
                    metrics
                        .get_counter(&format!("counter.task_{}", task_id))
                        .await,
                    increments_per_task
                );
            }
        }

        #[tokio::test]
        async fn test_concurrent_duration_recording() {
            let metrics = Arc::new(PersistenceMetrics::new());
            let num_tasks = 10;
            let durations_per_task = 100;

            let mut handles = vec![];

            for task_id in 0..num_tasks {
                let metrics_clone = metrics.clone();
                handles.push(tokio::spawn(async move {
                    for i in 0..durations_per_task {
                        // Record varied durations
                        let duration = Duration::from_millis((task_id * 10 + i % 10) as u64);
                        metrics_clone
                            .record_duration("operation.shared", duration)
                            .await;

                        // Task-specific operations
                        metrics_clone
                            .record_duration(
                                &format!("operation.task_{}", task_id),
                                Duration::from_millis(i as u64),
                            )
                            .await;
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            // Verify all durations were recorded
            let summary = metrics.summary().await;
            let shared_stats = summary.durations.get("operation.shared").unwrap();
            assert_eq!(
                shared_stats.count,
                (num_tasks * durations_per_task) as usize
            );

            // Verify percentiles make sense
            assert!(shared_stats.p50 < shared_stats.p95);
            assert!(shared_stats.p95 < shared_stats.p99);
            assert!(shared_stats.min <= shared_stats.p50);
            assert!(shared_stats.max >= shared_stats.p99);
        }

        #[tokio::test]
        async fn test_concurrent_error_recording() {
            let metrics = Arc::new(PersistenceMetrics::new());
            let error_count = Arc::new(AtomicU64::new(0));

            let mut handles = vec![];

            // Simulate operations with varying error rates
            for task_id in 0..20 {
                let metrics_clone = metrics.clone();
                let error_count_clone = error_count.clone();

                handles.push(tokio::spawn(async move {
                    for i in 0..50 {
                        if (task_id + i) % 3 == 0 {
                            // Record error
                            metrics_clone.record_error("operation.failing").await;
                            error_count_clone.fetch_add(1, Ordering::SeqCst);

                            // Also record task-specific error
                            metrics_clone
                                .record_error(&format!("task_{task_id}.error"))
                                .await;
                        } else {
                            // Record success
                            metrics_clone.increment("operation.success").await;
                        }
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            let expected_errors = error_count.load(Ordering::SeqCst);
            assert_eq!(
                metrics.get_error_count("operation.failing").await,
                expected_errors
            );

            // Verify success count
            let total_operations = 20 * 50;
            let success_count = metrics.get_counter("operation.success").await;
            assert_eq!(success_count + expected_errors, total_operations);
        }

        #[tokio::test]
        async fn test_concurrent_timer_usage() {
            let metrics = Arc::new(PersistenceMetrics::new());

            let mut handles = vec![];

            for task_id in 0..10 {
                let metrics_clone = metrics.clone();

                handles.push(tokio::spawn(async move {
                    for i in 0..20 {
                        // Simulate operation with timer
                        let timer = MetricsTimer::new(&metrics_clone, "timed.operation");

                        // Simulate work
                        tokio::time::sleep(Duration::from_micros(100)).await;

                        if (task_id + i) % 5 == 0 {
                            timer.record_error().await;
                        } else {
                            timer.record().await;
                        }
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            // Verify timer recorded both durations and counts
            assert_eq!(metrics.get_counter("timed.operation.count").await, 160); // 80% success
            assert_eq!(metrics.get_error_count("timed.operation").await, 40); // 20% error

            // Should have durations for all operations
            let avg_duration = metrics.get_avg_duration("timed.operation").await.unwrap();
            assert!(avg_duration >= Duration::from_micros(100));
        }

        #[tokio::test]
        async fn test_concurrent_summary_generation() {
            let metrics = Arc::new(PersistenceMetrics::new());

            // Start background tasks that continuously update metrics
            let mut update_handles = vec![];
            let stop_signal = Arc::new(AtomicU64::new(0));

            for i in 0..5 {
                let metrics_clone = metrics.clone();
                let stop_clone = stop_signal.clone();

                update_handles.push(tokio::spawn(async move {
                    while stop_clone.load(Ordering::Relaxed) == 0 {
                        metrics_clone.increment(&format!("counter_{i}")).await;
                        metrics_clone
                            .record_duration(&format!("op_{i}"), Duration::from_millis(i as u64))
                            .await;
                        tokio::task::yield_now().await;
                    }
                }));
            }

            // Concurrently generate summaries
            let mut summary_handles = vec![];
            for _ in 0..10 {
                let metrics_clone = metrics.clone();

                summary_handles.push(tokio::spawn(async move {
                    for _ in 0..20 {
                        let summary = metrics_clone.summary().await;
                        // Verify summary is consistent
                        assert!(summary.counters.len() >= 5);
                        assert!(summary.durations.len() >= 5);
                        tokio::task::yield_now().await;
                    }
                }));
            }

            // Let it run for a bit
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Stop updates
            stop_signal.store(1, Ordering::Relaxed);

            // Wait for all tasks
            for handle in update_handles {
                handle.await.unwrap();
            }
            for handle in summary_handles {
                handle.await.unwrap();
            }

            // Final summary should be consistent
            let final_summary = metrics.summary().await;
            assert_eq!(final_summary.counters.len(), 5);
            assert_eq!(final_summary.durations.len(), 5);
        }

        #[tokio::test]
        async fn test_concurrent_reset() {
            let metrics = Arc::new(PersistenceMetrics::new());

            // Fill with initial data
            for i in 0..100 {
                metrics.increment(&format!("counter_{}", i % 10)).await;
                metrics
                    .record_duration("op", Duration::from_millis(i))
                    .await;
                if i.is_multiple_of(3) {
                    metrics.record_error("op").await;
                }
            }

            // Verify data exists
            assert!(metrics.get_counter("counter_0").await > 0);
            assert!(metrics.get_avg_duration("op").await.is_some());
            assert!(metrics.get_error_count("op").await > 0);

            // Reset the metrics
            metrics.reset().await;

            // After reset, old counters should be 0
            assert_eq!(metrics.get_counter("counter_0").await, 0);
            assert!(metrics.get_avg_duration("op").await.is_none());
            assert_eq!(metrics.get_error_count("op").await, 0);

            // Test concurrent operations after reset
            let mut handles = vec![];
            for i in 0..5 {
                let metrics_clone = metrics.clone();
                handles.push(tokio::spawn(async move {
                    for _ in 0..20 {
                        metrics_clone.increment(&format!("new_counter_{i}")).await;
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            // New counters should exist
            for i in 0..5 {
                assert_eq!(metrics.get_counter(&format!("new_counter_{i}")).await, 20);
            }
        }

        #[tokio::test]
        async fn test_duration_buffer_limit() {
            let metrics = PersistenceMetrics::new();

            // Record more than 1000 durations sequentially to ensure order
            for i in 0..1500 {
                metrics
                    .record_duration("limited_op", Duration::from_millis(i as u64))
                    .await;
            }

            // Should only keep last 1000
            let summary = metrics.summary().await;
            let stats = summary.durations.get("limited_op").unwrap();
            assert_eq!(stats.count, 1000);

            // Min should be from later recordings (500-1499)
            assert!(stats.min >= Duration::from_millis(500));
            assert!(stats.max < Duration::from_millis(1500));
        }

        #[tokio::test]
        async fn test_high_contention_scenario() {
            let metrics = Arc::new(PersistenceMetrics::new());
            let operations = vec!["save", "load", "delete", "query", "update"];
            let num_tasks = 50;

            let mut handles = vec![];

            for task_id in 0..num_tasks {
                let metrics_clone = metrics.clone();
                let ops = operations.clone();

                handles.push(tokio::spawn(async move {
                    for i in 0..100 {
                        let op = ops[i % ops.len()];

                        // Start timer
                        let timer = MetricsTimer::new(&metrics_clone, op);

                        // Simulate work with varying duration
                        let work_time = Duration::from_micros((task_id + i) as u64 % 100);
                        tokio::time::sleep(work_time).await;

                        // Random failures
                        if (task_id * 100 + i).is_multiple_of(7) {
                            timer.record_error().await;
                        } else {
                            timer.record().await;
                        }

                        // Also update other metrics
                        metrics_clone.increment(&format!("{op}.attempts")).await;

                        // Occasionally read metrics
                        if i.is_multiple_of(10) {
                            let _ = metrics_clone.summary().await;
                        }
                    }
                }));
            }

            // Wait for completion
            for handle in handles {
                handle.await.unwrap();
            }

            // Verify metrics consistency
            let summary = metrics.summary().await;

            for op in operations {
                let attempts = summary.counters.get(&format!("{op}.attempts")).unwrap();
                let successes = summary.counters.get(&format!("{op}.count")).unwrap_or(&0);
                let errors = summary.errors.get(op).unwrap_or(&0);

                // Attempts should equal successes + errors
                assert_eq!(*attempts, successes + errors);

                // Should have duration stats
                assert!(summary.durations.contains_key(op));
            }
        }
    }
}
