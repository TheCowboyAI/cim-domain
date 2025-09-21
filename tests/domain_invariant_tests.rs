use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;

use cim_domain::composition::DomainComposition;
use cim_domain::domain::{DomainInvariant, InvariantChecker, InvariantViolation};
use cim_domain::DomainError;

struct AlwaysSatisfied;

#[async_trait]
impl DomainInvariant for AlwaysSatisfied {
    fn name(&self) -> &str {
        "always_satisfied"
    }

    fn description(&self) -> &str {
        "Invariant that is always satisfied"
    }

    async fn check(
        &self,
        _composition: &DomainComposition,
    ) -> Result<cim_domain::domain::invariants::InvariantCheckResult, DomainError> {
        Ok(cim_domain::domain::invariants::InvariantCheckResult {
            satisfied: true,
            violations: Vec::new(),
            checked_at: Utc::now(),
            context: HashMap::new(),
        })
    }

    fn affected_domains(&self) -> Vec<String> {
        vec!["person".to_string()]
    }
}

struct AlwaysViolates;

#[async_trait]
impl DomainInvariant for AlwaysViolates {
    fn name(&self) -> &str {
        "always_violates"
    }

    fn description(&self) -> &str {
        "Invariant that always produces a violation"
    }

    async fn check(
        &self,
        _composition: &DomainComposition,
    ) -> Result<cim_domain::domain::invariants::InvariantCheckResult, DomainError> {
        let violation = InvariantViolation {
            invariant_name: self.name().to_string(),
            location: cim_domain::domain::invariants::ViolationLocation::Domain {
                name: "policy".to_string(),
            },
            message: "policy threshold exceeded".to_string(),
            severity: cim_domain::domain::invariants::ViolationSeverity::Error,
            remediation: Some("review policy inputs".to_string()),
        };

        Ok(cim_domain::domain::invariants::InvariantCheckResult {
            satisfied: false,
            violations: vec![violation],
            checked_at: Utc::now(),
            context: HashMap::new(),
        })
    }

    fn affected_domains(&self) -> Vec<String> {
        vec!["policy".to_string()]
    }
}

#[tokio::test]
async fn invariant_checker_records_results() {
    let mut checker = InvariantChecker::new();
    checker.register(Box::new(AlwaysSatisfied));
    checker.register(Box::new(AlwaysViolates));

    let composition = DomainComposition::new("person-policy".to_string());
    let results = checker
        .check_all(&composition)
        .await
        .expect("check invariants");

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.satisfied));
    assert!(results.iter().any(|r| !r.satisfied));

    // Checker retains history internally; length verified via results len when re-running.
}
