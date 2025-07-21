// Copyright 2025 Cowboy AI, LLC.

//! Query optimization using NATS subject-based routing and indexing

use crate::DomainError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::subject_abstraction::{Subject, Pattern};
#[cfg(feature = "subject-routing")]
use cim_subject::SubjectAlgebra;

/// Query performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformance {
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of subjects scanned
    pub subjects_scanned: usize,
    /// Number of results returned
    pub results_returned: usize,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Index usage
    pub indexes_used: Vec<String>,
}

/// Index strategy for optimization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexStrategy {
    /// Use subject pattern matching
    SubjectPattern,
    /// Use KV store index
    KeyValue,
    /// Use time-based index
    TimeSeries,
    /// Use composite index
    Composite(Vec<String>),
    /// Full scan (no index)
    FullScan,
}

/// Query hint for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHint {
    /// Preferred index strategy
    pub strategy: Option<IndexStrategy>,
    /// Expected result size
    pub expected_size: Option<usize>,
    /// Time range for filtering
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Fields to include in results
    pub projection: Option<Vec<String>>,
    /// Whether to use cache
    pub use_cache: bool,
}

impl Default for QueryHint {
    fn default() -> Self {
        Self {
            strategy: None,
            expected_size: None,
            time_range: None,
            projection: None,
            use_cache: true,
        }
    }
}

/// Query execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Query ID for tracking
    pub query_id: String,
    /// Steps in the execution plan
    pub steps: Vec<QueryStep>,
    /// Estimated cost
    pub estimated_cost: f64,
    /// Selected strategy
    pub strategy: IndexStrategy,
    /// Subject patterns to scan
    pub subject_patterns: Vec<Pattern>,
}

/// Individual step in query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStep {
    /// Step name
    pub name: String,
    /// Operation type
    pub operation: QueryOperation,
    /// Estimated rows
    pub estimated_rows: usize,
    /// Cost estimate
    pub cost: f64,
}

/// Query operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryOperation {
    /// Scan subjects matching pattern
    SubjectScan(String),
    /// Filter by predicate
    Filter(String),
    /// Join with another dataset
    Join(String),
    /// Sort results
    Sort(Vec<String>),
    /// Limit results
    Limit(usize),
    /// Project specific fields
    Project(Vec<String>),
}

/// Subject-based index for fast lookups
#[derive(Debug, Clone)]
pub struct SubjectIndex {
    /// Index name
    pub name: String,
    /// Indexed patterns
    patterns: Arc<RwLock<HashMap<String, HashSet<Subject>>>>,
    /// Reverse index (subject to patterns)
    reverse_index: Arc<RwLock<HashMap<Subject, HashSet<String>>>>,
    /// Statistics
    stats: Arc<RwLock<IndexStats>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct IndexStats {
    total_subjects: usize,
    total_patterns: usize,
    last_updated: Option<DateTime<Utc>>,
    query_count: u64,
    hit_count: u64,
}

impl SubjectIndex {
    /// Create a new subject index
    pub fn new(name: String) -> Self {
        Self {
            name,
            patterns: Arc::new(RwLock::new(HashMap::new())),
            reverse_index: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(IndexStats::default())),
        }
    }
    
    /// Add a subject to the index
    pub async fn add_subject(&self, subject: Subject, patterns: Vec<String>) {
        let mut pattern_index = self.patterns.write().await;
        let mut reverse = self.reverse_index.write().await;
        
        for pattern_str in patterns {
            pattern_index
                .entry(pattern_str.clone())
                .or_insert_with(HashSet::new)
                .insert(subject.clone());
            
            reverse
                .entry(subject.clone())
                .or_insert_with(HashSet::new)
                .insert(pattern_str);
        }
        
        let mut stats = self.stats.write().await;
        stats.total_subjects = reverse.len();
        stats.total_patterns = pattern_index.len();
        stats.last_updated = Some(Utc::now());
    }
    
    /// Find subjects matching a pattern
    pub async fn find_by_pattern(&self, pattern: &Pattern) -> Vec<Subject> {
        let patterns = self.patterns.read().await;
        let mut stats = self.stats.write().await;
        stats.query_count += 1;
        
        let mut results = HashSet::new();
        
        // Check all indexed subjects
        for (pattern_str, subjects) in patterns.iter() {
            if pattern.matches(&Subject::new(pattern_str).unwrap()) {
                stats.hit_count += 1;
                results.extend(subjects.clone());
            }
        }
        
        results.into_iter().collect()
    }
    
    /// Get index statistics
    pub async fn get_stats(&self) -> IndexStats {
        self.stats.read().await.clone()
    }
}

/// Query optimizer for NATS-based persistence
#[async_trait]
pub trait QueryOptimizer: Send + Sync {
    /// Create a query plan
    async fn create_plan(
        &self,
        query: &str,
        hints: QueryHint,
    ) -> Result<QueryPlan, DomainError>;
    
    /// Execute a query plan
    async fn execute_plan(
        &self,
        plan: &QueryPlan,
    ) -> Result<(Vec<serde_json::Value>, QueryPerformance), DomainError>;
    
    /// Analyze query patterns for optimization
    async fn analyze_patterns(
        &self,
        queries: Vec<String>,
    ) -> Result<HashMap<String, f64>, DomainError>;
    
    /// Get or create index
    async fn get_index(&self, name: &str) -> Option<SubjectIndex>;
    
    /// Create a new index
    async fn create_index(
        &self,
        name: String,
        patterns: Vec<String>,
    ) -> Result<SubjectIndex, DomainError>;
    
    /// Update query statistics
    async fn update_stats(&self, plan: &QueryPlan, performance: &QueryPerformance);
}

/// NATS-based query optimizer implementation
pub struct NatsQueryOptimizer {
    /// Subject algebra for query optimization
    #[cfg(feature = "subject-routing")]
    algebra: SubjectAlgebra,
    /// Available indexes
    indexes: Arc<RwLock<HashMap<String, SubjectIndex>>>,
    /// Query statistics
    query_stats: Arc<RwLock<HashMap<String, QueryStats>>>,
    /// Cache for query results
    result_cache: Arc<RwLock<HashMap<String, CachedResult>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryStats {
    query_pattern: String,
    execution_count: u64,
    avg_execution_time: f64,
    avg_results: f64,
    last_executed: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedResult {
    data: Vec<serde_json::Value>,
    cached_at: DateTime<Utc>,
    ttl_seconds: u64,
}

impl NatsQueryOptimizer {
    /// Create a new query optimizer
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "subject-routing")]
            algebra: SubjectAlgebra::new(),
            indexes: Arc::new(RwLock::new(HashMap::new())),
            query_stats: Arc::new(RwLock::new(HashMap::new())),
            result_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Parse query into subject patterns
    fn parse_query(&self, query: &str) -> Result<Vec<Pattern>, DomainError> {
        // This is a simplified query parser
        // In a real implementation, you'd parse a query language
        let patterns = query
            .split(" AND ")
            .filter_map(|part| {
                if part.contains("subject:") {
                    let pattern_str = part.replace("subject:", "").trim().to_string();
                    Pattern::new(&pattern_str).ok()
                } else {
                    None
                }
            })
            .collect();
        
        Ok(patterns)
    }
    
    /// Estimate query cost
    fn estimate_cost(&self, strategy: &IndexStrategy, patterns: &[Pattern]) -> f64 {
        match strategy {
            IndexStrategy::SubjectPattern => 1.0 * patterns.len() as f64,
            IndexStrategy::KeyValue => 2.0,
            IndexStrategy::TimeSeries => 1.5,
            IndexStrategy::Composite(fields) => 1.2 * fields.len() as f64,
            IndexStrategy::FullScan => 100.0,
        }
    }
    
    /// Select best strategy based on query
    async fn select_strategy(&self, patterns: &[Pattern], hints: &QueryHint) -> IndexStrategy {
        if let Some(strategy) = &hints.strategy {
            return strategy.clone();
        }
        
        // Check available indexes
        let indexes = self.indexes.read().await;
        
        // If we have subject patterns and matching indexes, use them
        if !patterns.is_empty() && !indexes.is_empty() {
            return IndexStrategy::SubjectPattern;
        }
        
        // If time range specified, use time series
        if hints.time_range.is_some() {
            return IndexStrategy::TimeSeries;
        }
        
        // Default to KV if available
        IndexStrategy::KeyValue
    }
}

#[async_trait]
impl QueryOptimizer for NatsQueryOptimizer {
    async fn create_plan(
        &self,
        query: &str,
        hints: QueryHint,
    ) -> Result<QueryPlan, DomainError> {
        // Check cache first if enabled
        if hints.use_cache {
            let cache = self.result_cache.read().await;
            if let Some(cached) = cache.get(query) {
                let age = Utc::now().signed_duration_since(cached.cached_at);
                if age.num_seconds() < cached.ttl_seconds as i64 {
                    // Return cached plan
                    return Ok(QueryPlan {
                        query_id: uuid::Uuid::new_v4().to_string(),
                        steps: vec![QueryStep {
                            name: "CacheHit".to_string(),
                            operation: QueryOperation::Filter("cached".to_string()),
                            estimated_rows: cached.data.len(),
                            cost: 0.1,
                        }],
                        estimated_cost: 0.1,
                        strategy: IndexStrategy::SubjectPattern,
                        subject_patterns: vec![],
                    });
                }
            }
        }
        
        // Parse query into patterns
        let patterns = self.parse_query(query)?;
        
        // Select strategy
        let strategy = self.select_strategy(&patterns, &hints).await;
        
        // Build query steps
        let mut steps = Vec::new();
        let mut total_cost = 0.0;
        
        // Add subject scan step
        if !patterns.is_empty() {
            for (i, pattern) in patterns.iter().enumerate() {
                let cost = 1.0;
                steps.push(QueryStep {
                    name: format!("SubjectScan{}", i),
                    operation: QueryOperation::SubjectScan(pattern.to_string()),
                    estimated_rows: hints.expected_size.unwrap_or(100),
                    cost,
                });
                total_cost += cost;
            }
        }
        
        // Add projection if specified
        if let Some(fields) = &hints.projection {
            steps.push(QueryStep {
                name: "Project".to_string(),
                operation: QueryOperation::Project(fields.clone()),
                estimated_rows: hints.expected_size.unwrap_or(100),
                cost: 0.5,
            });
            total_cost += 0.5;
        }
        
        // Add limit if specified
        if let Some(size) = hints.expected_size {
            steps.push(QueryStep {
                name: "Limit".to_string(),
                operation: QueryOperation::Limit(size),
                estimated_rows: size,
                cost: 0.1,
            });
            total_cost += 0.1;
        }
        
        Ok(QueryPlan {
            query_id: uuid::Uuid::new_v4().to_string(),
            steps,
            estimated_cost: total_cost,
            strategy,
            subject_patterns: patterns,
        })
    }
    
    async fn execute_plan(
        &self,
        plan: &QueryPlan,
    ) -> Result<(Vec<serde_json::Value>, QueryPerformance), DomainError> {
        let start = std::time::Instant::now();
        let mut results = Vec::new();
        let mut subjects_scanned = 0;
        let mut indexes_used = Vec::new();
        
        // Execute each step
        for step in &plan.steps {
            match &step.operation {
                QueryOperation::SubjectScan(pattern_str) => {
                    let pattern = Pattern::new(pattern_str)
                        .map_err(|e| DomainError::InvalidOperation {
                            reason: format!("Invalid pattern: {}", e),
                        })?;
                    
                    // Check if we have an index for this pattern
                    let indexes = self.indexes.read().await;
                    for (name, index) in indexes.iter() {
                        let matching_subjects = index.find_by_pattern(&pattern).await;
                        subjects_scanned += matching_subjects.len();
                        indexes_used.push(name.clone());
                        
                        // Convert subjects to results (simplified)
                        for subject in matching_subjects {
                            results.push(serde_json::json!({
                                "subject": subject.to_string(),
                                "timestamp": Utc::now(),
                            }));
                        }
                    }
                }
                QueryOperation::Filter(predicate) => {
                    // Apply filter (simplified)
                    results.retain(|r| {
                        r.get("subject")
                            .and_then(|s| s.as_str())
                            .map(|s| s.contains(predicate))
                            .unwrap_or(false)
                    });
                }
                QueryOperation::Project(fields) => {
                    // Project only specified fields
                    results = results.into_iter().map(|mut r| {
                        if let Some(obj) = r.as_object_mut() {
                            obj.retain(|k, _| fields.contains(&k.to_string()));
                        }
                        r
                    }).collect();
                }
                QueryOperation::Limit(size) => {
                    results.truncate(*size);
                }
                _ => {}
            }
        }
        
        let execution_time = start.elapsed().as_millis() as u64;
        
        let performance = QueryPerformance {
            execution_time_ms: execution_time,
            subjects_scanned,
            results_returned: results.len(),
            cache_hit_rate: 0.0, // Calculate based on actual cache usage
            indexes_used,
        };
        
        Ok((results, performance))
    }
    
    async fn analyze_patterns(
        &self,
        queries: Vec<String>,
    ) -> Result<HashMap<String, f64>, DomainError> {
        let mut pattern_scores = HashMap::new();
        
        for query in queries {
            let patterns = self.parse_query(&query)?;
            for pattern in patterns {
                let score = pattern_scores.entry(pattern.to_string()).or_insert(0.0);
                *score += 1.0;
            }
        }
        
        // Normalize scores
        let total: f64 = pattern_scores.values().sum();
        if total > 0.0 {
            for score in pattern_scores.values_mut() {
                *score /= total;
            }
        }
        
        Ok(pattern_scores)
    }
    
    async fn get_index(&self, name: &str) -> Option<SubjectIndex> {
        self.indexes.read().await.get(name).cloned()
    }
    
    async fn create_index(
        &self,
        name: String,
        patterns: Vec<String>,
    ) -> Result<SubjectIndex, DomainError> {
        let index = SubjectIndex::new(name.clone());
        
        // Pre-populate index with patterns
        for pattern_str in patterns {
            let pattern = Pattern::new(&pattern_str)
                .map_err(|e| DomainError::InvalidOperation {
                    reason: format!("Invalid pattern: {}", e),
                })?;
            
            // This would scan existing data and populate the index
            // For now, we'll just create the empty index
        }
        
        self.indexes.write().await.insert(name.clone(), index.clone());
        
        Ok(index)
    }
    
    async fn update_stats(&self, plan: &QueryPlan, performance: &QueryPerformance) {
        let mut stats = self.query_stats.write().await;
        
        let query_pattern = plan.subject_patterns
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" AND ");
        
        let entry = stats.entry(query_pattern.clone()).or_insert(QueryStats {
            query_pattern,
            execution_count: 0,
            avg_execution_time: 0.0,
            avg_results: 0.0,
            last_executed: Utc::now(),
        });
        
        // Update rolling averages
        let count = entry.execution_count as f64;
        entry.avg_execution_time = (entry.avg_execution_time * count + performance.execution_time_ms as f64) / (count + 1.0);
        entry.avg_results = (entry.avg_results * count + performance.results_returned as f64) / (count + 1.0);
        entry.execution_count += 1;
        entry.last_executed = Utc::now();
    }
}