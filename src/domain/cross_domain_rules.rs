// Copyright (c) 2025 - Cowboy AI, LLC.

//! Cross-domain business rules
//!
//! Rules that span multiple domains and must be enforced consistently
//! across domain boundaries.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::composition::DomainComposition;
use crate::errors::DomainError;

/// A cross-domain business rule
#[async_trait]
pub trait CrossDomainRule: Send + Sync {
    /// Name of the rule
    fn name(&self) -> &str;

    /// Description of the rule
    fn description(&self) -> &str;

    /// Evaluate the rule against a domain composition
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        context: &RuleContext,
    ) -> Result<RuleEvaluationResult, DomainError>;

    /// Get the domains this rule affects
    fn affected_domains(&self) -> Vec<String>;

    /// Get the priority of this rule (higher = more important)
    fn priority(&self) -> u32 {
        50 // Default medium priority
    }
}

/// Context for rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleContext {
    /// Current user/actor
    pub actor: Option<String>,

    /// Current operation being performed
    pub operation: Option<String>,

    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,

    /// Timestamp of evaluation
    pub timestamp: DateTime<Utc>,
}

impl Default for RuleContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleContext {
    /// Create a new rule context
    pub fn new() -> Self {
        Self {
            actor: None,
            operation: None,
            data: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Set the actor for this context
    pub fn with_actor(mut self, actor: String) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Set the operation for this context
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Add data to the context
    pub fn with_data(mut self, key: String, value: serde_json::Value) -> Self {
        self.data.insert(key, value);
        self
    }
}

/// Result of evaluating a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluationResult {
    /// Whether the rule passed
    pub passed: bool,

    /// Confidence level (0-100)
    pub confidence: u8,

    /// Explanation of the result
    pub explanation: String,

    /// Actions to take based on the result
    pub actions: Vec<RuleAction>,

    /// Metadata about the evaluation
    pub metadata: HashMap<String, String>,
}

/// Actions that can be taken based on rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Allow the operation to proceed
    Allow,

    /// Deny the operation
    Deny {
        /// Reason for denial
        reason: String,
    },

    /// Require additional approval
    RequireApproval {
        /// Role that must approve
        approver_role: String,
    },

    /// Log the event for audit
    Log {
        /// Log level for the message
        level: LogLevel,
        /// Log message content
        message: String,
    },

    /// Send notification
    Notify {
        /// Notification recipient
        recipient: String,
        /// Notification message
        message: String,
    },

    /// Execute compensation
    Compensate {
        /// Compensation action to execute
        action: String,
    },

    /// Custom action
    Custom {
        /// Type of custom action
        action_type: String,
        /// Additional action data
        data: serde_json::Value,
    },
}

/// Log levels for rule actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    /// Debug level logging
    Debug,
    /// Informational logging
    Info,
    /// Warning level logging
    Warning,
    /// Error level logging
    Error,
    /// Critical level logging
    Critical,
}

/// Engine for evaluating cross-domain rules
pub struct RuleEngine {
    /// Registered rules
    rules: Vec<Box<dyn CrossDomainRule>>,

    /// Rule evaluation history
    history: Vec<RuleEvaluationRecord>,

    /// Rule caching
    cache: HashMap<String, RuleEvaluationResult>,
}

/// Record of a rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluationRecord {
    /// Name of the evaluated rule
    pub rule_name: String,
    /// Context in which the rule was evaluated
    pub context: RuleContext,
    /// Result of the evaluation
    pub result: RuleEvaluationResult,
    /// When the evaluation occurred
    pub evaluated_at: DateTime<Utc>,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleEngine {
    /// Create a new rule engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            history: Vec::new(),
            cache: HashMap::new(),
        }
    }

    /// Register a rule
    pub fn register(&mut self, rule: Box<dyn CrossDomainRule>) {
        self.rules.push(rule);
        // Sort by priority (highest first)
        self.rules.sort_by_key(|r| std::cmp::Reverse(r.priority()));
    }

    /// Evaluate all rules
    pub async fn evaluate_all(
        &mut self,
        composition: &DomainComposition,
        context: &RuleContext,
    ) -> Result<Vec<RuleEvaluationResult>, DomainError> {
        let mut results = Vec::new();

        for rule in &self.rules {
            let cache_key = format!("{}:{:?}", rule.name(), context.operation);

            // Check cache
            if let Some(cached) = self.cache.get(&cache_key) {
                results.push(cached.clone());
                continue;
            }

            // Evaluate rule
            let result = rule.evaluate(composition, context).await?;

            // Record evaluation
            self.history.push(RuleEvaluationRecord {
                rule_name: rule.name().to_string(),
                context: context.clone(),
                result: result.clone(),
                evaluated_at: Utc::now(),
            });

            // Cache result
            self.cache.insert(cache_key, result.clone());

            // Always push result before checking for short-circuit
            results.push(result.clone());

            // Short-circuit on critical deny
            if matches!(result.actions.first(), Some(RuleAction::Deny { .. }))
                && rule.priority() >= 90
            {
                break;
            }
        }

        Ok(results)
    }

    /// Get all deny actions from results
    pub fn get_denials(results: &[RuleEvaluationResult]) -> Vec<&str> {
        results
            .iter()
            .flat_map(|r| &r.actions)
            .filter_map(|a| match a {
                RuleAction::Deny { reason } => Some(reason.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Check if operation is allowed
    pub fn is_allowed(results: &[RuleEvaluationResult]) -> bool {
        Self::get_denials(results).is_empty()
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

/// Example: Data locality rule
pub struct DataLocalityRule {
    /// Map of domain names to their required geographical locations
    required_locality: HashMap<String, String>, // domain -> required location
}

impl Default for DataLocalityRule {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLocalityRule {
    /// Create a new data locality rule with default requirements
    pub fn new() -> Self {
        let mut required = HashMap::new();
        required.insert("PersonalData".to_string(), "EU".to_string());
        required.insert("FinancialData".to_string(), "US".to_string());

        Self {
            required_locality: required,
        }
    }
}

#[async_trait]
impl CrossDomainRule for DataLocalityRule {
    fn name(&self) -> &str {
        "data_locality"
    }

    fn description(&self) -> &str {
        "Ensures data is stored in required geographical locations"
    }

    async fn evaluate(
        &self,
        _composition: &DomainComposition,
        context: &RuleContext,
    ) -> Result<RuleEvaluationResult, DomainError> {
        let mut violations = Vec::new();

        // Check each domain's data location
        for (domain_name, required_location) in &self.required_locality {
            let location_key = format!("{domain_name}_location");
            if let Some(actual_location) = context.data.get(&location_key) {
                if actual_location.as_str() != Some(required_location.as_str()) {
                    violations.push(format!(
                        "{domain_name} must be in {required_location}, but is in {actual_location:?}"
                    ));
                }
            } else if let Some(ref op) = context.operation {
                if op.contains("read") || op.contains("write") {
                    // If we're doing data operations, we should know the location
                    violations.push(format!(
                        "{domain_name} location not specified for {op} operation"
                    ));
                }
            }
        }

        let passed = violations.is_empty();
        let actions = if passed {
            vec![RuleAction::Allow]
        } else {
            vec![
                RuleAction::Deny {
                    reason: violations.join("; "),
                },
                RuleAction::Log {
                    level: LogLevel::Error,
                    message: format!("Data locality violations: {}", violations.join(", ")),
                },
            ]
        };

        let result = RuleEvaluationResult {
            passed,
            confidence: 100,
            explanation: if passed {
                "All data locality requirements satisfied".to_string()
            } else {
                format!("Data locality violations found: {}", violations.len())
            },
            actions,
            metadata: HashMap::new(),
        };
        Ok(result)
    }

    fn affected_domains(&self) -> Vec<String> {
        self.required_locality.keys().cloned().collect()
    }

    fn priority(&self) -> u32 {
        90 // High priority for compliance
    }
}

/// Example: Transaction consistency rule
pub struct TransactionConsistencyRule {
    /// Maximum allowed inconsistency window in milliseconds
    max_inconsistency_window_ms: u64,
}

impl TransactionConsistencyRule {
    /// Create a new transaction consistency rule
    pub fn new(max_window_ms: u64) -> Self {
        Self {
            max_inconsistency_window_ms: max_window_ms,
        }
    }
}

#[async_trait]
impl CrossDomainRule for TransactionConsistencyRule {
    fn name(&self) -> &str {
        "transaction_consistency"
    }

    fn description(&self) -> &str {
        "Ensures cross-domain transactions maintain consistency"
    }

    async fn evaluate(
        &self,
        _composition: &DomainComposition,
        context: &RuleContext,
    ) -> Result<RuleEvaluationResult, DomainError> {
        // Check if this is a transaction operation
        let is_transaction = context
            .operation
            .as_ref()
            .map(|op| op.contains("transaction") || op.contains("transfer"))
            .unwrap_or(false);

        if !is_transaction {
            return Ok(RuleEvaluationResult {
                passed: true,
                confidence: 100,
                explanation: "Not a transaction operation".to_string(),
                actions: vec![RuleAction::Allow],
                metadata: HashMap::new(),
            });
        }

        // In real implementation, would check:
        // 1. All participating domains support transactions
        // 2. Consistency window is acceptable
        // 3. Compensation handlers are available

        Ok(RuleEvaluationResult {
            passed: true,
            confidence: 95,
            explanation: "Transaction consistency requirements met".to_string(),
            actions: vec![
                RuleAction::Allow,
                RuleAction::Log {
                    level: LogLevel::Info,
                    message: "Cross-domain transaction initiated".to_string(),
                },
            ],
            metadata: HashMap::from([(
                "max_window_ms".to_string(),
                self.max_inconsistency_window_ms.to_string(),
            )]),
        })
    }

    fn affected_domains(&self) -> Vec<String> {
        vec!["*".to_string()] // Affects all domains
    }

    fn priority(&self) -> u32 {
        80
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rule_engine() {
        let mut engine = RuleEngine::new();

        // Register rules
        engine.register(Box::new(DataLocalityRule::new()));
        engine.register(Box::new(TransactionConsistencyRule::new(5000)));

        // Create test composition
        let composition = DomainComposition::new("Test".to_string());

        // Create context with proper data locality
        let context = RuleContext::new()
            .with_operation("read_data".to_string())
            .with_data("PersonalData_location".to_string(), serde_json::json!("EU"))
            .with_data(
                "FinancialData_location".to_string(),
                serde_json::json!("US"),
            );

        let results = engine.evaluate_all(&composition, &context).await.unwrap();

        assert!(RuleEngine::is_allowed(&results));

        // Test with violation
        let bad_context = RuleContext::new()
            .with_operation("read_data".to_string())
            .with_data("PersonalData_location".to_string(), serde_json::json!("US"))
            .with_data(
                "FinancialData_location".to_string(),
                serde_json::json!("US"),
            ); // Add this to avoid missing location error

        // Clear cache to ensure fresh evaluation
        engine.clear_cache();

        let results = engine
            .evaluate_all(&composition, &bad_context)
            .await
            .unwrap();

        // Should be denied because PersonalData must be in EU but is in US
        let denials = RuleEngine::get_denials(&results);
        assert!(
            !denials.is_empty(),
            "Expected denials but got none. Results: {:?}",
            results
        );
        assert!(!RuleEngine::is_allowed(&results));
    }

    #[test]
    fn test_rule_priority() {
        let mut engine = RuleEngine::new();

        // Lower priority rule
        engine.register(Box::new(TransactionConsistencyRule::new(5000)));
        // Higher priority rule
        engine.register(Box::new(DataLocalityRule::new()));

        // Check rules are sorted by priority
        assert_eq!(engine.rules[0].priority(), 90);
        assert_eq!(engine.rules[1].priority(), 80);
    }
}
