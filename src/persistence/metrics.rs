//! Metrics collection for persistence operations

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;

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
        durations.entry(operation.to_string())
            .or_insert_with(Vec::new)
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
    pub async fn get_percentile_duration(&self, operation: &str, percentile: f64) -> Option<Duration> {
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
                
                duration_stats.insert(op.clone(), DurationStats {
                    count: vec.len(),
                    avg,
                    p50: sorted[p50_idx],
                    p95: sorted[p95_idx],
                    p99: sorted[p99_idx],
                    min: *sorted.first().unwrap(),
                    max: *sorted.last().unwrap(),
                });
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
        self.metrics.record_duration(&self.operation, duration).await;
        self.metrics.increment(&format!("{}.count", self.operation)).await;
    }
    
    /// Record as error
    pub async fn record_error(self) {
        let duration = self.start.elapsed();
        self.metrics.record_duration(&self.operation, duration).await;
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
        
        metrics.record_duration("op1", Duration::from_millis(10)).await;
        metrics.record_duration("op1", Duration::from_millis(20)).await;
        metrics.record_duration("op1", Duration::from_millis(30)).await;
        
        let avg = metrics.get_avg_duration("op1").await.unwrap();
        assert_eq!(avg, Duration::from_millis(20));
        
        let p50 = metrics.get_percentile_duration("op1", 50.0).await.unwrap();
        assert_eq!(p50, Duration::from_millis(20));
    }
    
    #[tokio::test]
    async fn test_timer() {
        let metrics = PersistenceMetrics::new();
        
        {
            let timer = metrics.timer("test.operation");
            tokio::time::sleep(Duration::from_millis(10)).await;
            timer.record().await;
        }
        
        assert_eq!(metrics.get_counter("test.operation.count").await, 1);
        assert!(metrics.get_avg_duration("test.operation").await.unwrap() >= Duration::from_millis(10));
    }
    
    #[tokio::test]
    async fn test_summary() {
        let metrics = PersistenceMetrics::new();
        
        metrics.increment("saves").await;
        metrics.record_error("load").await;
        metrics.record_duration("save", Duration::from_millis(5)).await;
        metrics.record_duration("save", Duration::from_millis(15)).await;
        
        let summary = metrics.summary().await;
        
        assert_eq!(summary.counters.get("saves"), Some(&1));
        assert_eq!(summary.errors.get("load"), Some(&1));
        assert_eq!(summary.durations.get("save").unwrap().count, 2);
        assert_eq!(summary.durations.get("save").unwrap().avg, Duration::from_millis(10));
    }
}