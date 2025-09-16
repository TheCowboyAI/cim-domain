// Copyright 2025 Cowboy AI, LLC.

//! Error types for domain operations

use thiserror::Error;

/// Errors that can occur in domain operations
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    // Component-related errors removed in pure-library scope
    /// Entity not found
    #[error("Entity not found: {entity_type} with id {id}")]
    EntityNotFound {
        /// Type of entity that wasn't found
        entity_type: String,
        /// ID that was searched for
        id: String,
    },

    /// Invalid operation
    #[error("Invalid operation: {reason}")]
    InvalidOperation {
        /// Reason why the operation is invalid
        reason: String,
    },

    /// Invariant violation
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    /// Aggregate not found
    #[error("Aggregate not found: {0}")]
    AggregateNotFound(String),

    /// Invalid state transition
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition {
        /// Current state
        from: String,
        /// Attempted target state
        to: String,
    },

    /// Concurrency conflict
    #[error("Concurrency conflict: expected version {expected}, but found {actual}")]
    ConcurrencyConflict {
        /// Expected version
        expected: u64,
        /// Actual version
        actual: u64,
    },

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Authorization error
    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    /// Business rule violation
    #[error("Business rule violation: {rule}")]
    BusinessRuleViolation {
        /// Description of the violated rule
        rule: String,
    },

    /// Context boundary violation
    #[error("Context boundary violation: {0}")]
    ContextBoundaryViolation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// External service error
    #[error("External service error: {service} - {message}")]
    ExternalServiceError {
        /// Name of the external service
        service: String,
        /// Error message from the service
        message: String,
    },

    /// Generic domain error
    #[error("Domain error: {0}")]
    Generic(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    InternalError(String),

    // Subject parsing errors removed (transport concern)

    // Integration errors removed in pure-library scope
    /// Not found error (generic)
    #[error("Not found: {0}")]
    NotFound(String),

    /// Already exists error (generic)
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// Not implemented error
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Result type for domain operations
pub type DomainResult<T> = Result<T, DomainError>;

impl From<serde_json::Error> for DomainError {
    fn from(err: serde_json::Error) -> Self {
        DomainError::SerializationError(err.to_string())
    }
}

impl DomainError {
    /// Create a generic domain error
    pub fn generic(msg: impl Into<String>) -> Self {
        DomainError::Generic(msg.into())
    }

    /// Check if this is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            DomainError::EntityNotFound { .. } | DomainError::AggregateNotFound(_)
        )
    }

    /// Check if this is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            DomainError::ValidationError(_)
                | DomainError::InvariantViolation(_)
                | DomainError::BusinessRuleViolation { .. }
        )
    }

    /// Check if this is a concurrency error
    pub fn is_concurrency_error(&self) -> bool {
        matches!(self, DomainError::ConcurrencyConflict { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test error creation and display messages
    ///
    /// ```mermaid
    /// graph TD
    ///     A[DomainError] -->|Display| B[Error Message]
    ///     A -->|Clone| C[Cloned Error]
    ///     A -->|Debug| D[Debug Format]
    /// ```
    #[test]
    fn test_error_display_messages() {
        // Component-specific errors removed

        // Test EntityNotFound
        let err = DomainError::EntityNotFound {
            entity_type: "User".to_string(),
            id: "123".to_string(),
        };
        assert_eq!(err.to_string(), "Entity not found: User with id 123");

        // Test InvalidOperation
        let err = DomainError::InvalidOperation {
            reason: "Cannot delete root entity".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid operation: Cannot delete root entity"
        );

        // Test InvariantViolation
        let err = DomainError::InvariantViolation("Balance cannot be negative".to_string());
        assert_eq!(
            err.to_string(),
            "Invariant violation: Balance cannot be negative"
        );

        // Test AggregateNotFound
        let err = DomainError::AggregateNotFound("Order-456".to_string());
        assert_eq!(err.to_string(), "Aggregate not found: Order-456");

        // Test InvalidStateTransition
        let err = DomainError::InvalidStateTransition {
            from: "Pending".to_string(),
            to: "Completed".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid state transition from Pending to Completed"
        );

        // Test ConcurrencyConflict
        let err = DomainError::ConcurrencyConflict {
            expected: 5,
            actual: 3,
        };
        assert_eq!(
            err.to_string(),
            "Concurrency conflict: expected version 5, but found 3"
        );

        // Test ValidationError
        let err = DomainError::ValidationError("Email format invalid".to_string());
        assert_eq!(err.to_string(), "Validation error: Email format invalid");

        // Test AuthorizationError
        let err = DomainError::AuthorizationError("Insufficient permissions".to_string());
        assert_eq!(
            err.to_string(),
            "Authorization error: Insufficient permissions"
        );

        // Test BusinessRuleViolation
        let err = DomainError::BusinessRuleViolation {
            rule: "Order minimum is $10".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Business rule violation: Order minimum is $10"
        );

        // Test ContextBoundaryViolation
        let err = DomainError::ContextBoundaryViolation(
            "Direct access to Order aggregate from Inventory context".to_string(),
        );
        assert_eq!(
            err.to_string(),
            "Context boundary violation: Direct access to Order aggregate from Inventory context"
        );

        // Test SerializationError
        let err = DomainError::SerializationError("Invalid JSON".to_string());
        assert_eq!(err.to_string(), "Serialization error: Invalid JSON");

        // Test ExternalServiceError
        let err = DomainError::ExternalServiceError {
            service: "PaymentGateway".to_string(),
            message: "Connection timeout".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "External service error: PaymentGateway - Connection timeout"
        );

        // Test Generic
        let err = DomainError::Generic("Something went wrong".to_string());
        assert_eq!(err.to_string(), "Domain error: Something went wrong");

        // Test InternalError
        let err = DomainError::InternalError("Unexpected state".to_string());
        assert_eq!(err.to_string(), "Internal error: Unexpected state");

        // Subject parsing errors are transport concerns
    }

    /// Test error cloning
    #[test]
    fn test_error_clone() {
        let original = DomainError::ValidationError("Test error".to_string());
        let cloned = original.clone();

        assert_eq!(original.to_string(), cloned.to_string());
    }

    /// Test generic error constructor
    #[test]
    fn test_generic_constructor() {
        let err1 = DomainError::generic("Test message");
        assert_eq!(err1.to_string(), "Domain error: Test message");

        let err2 = DomainError::generic(String::from("Another message"));
        assert_eq!(err2.to_string(), "Domain error: Another message");
    }

    /// Test is_not_found helper
    ///
    /// ```mermaid
    /// graph TD
    ///     A[EntityNotFound] -->|is_not_found| B[true]
    ///     C[ComponentNotFound] -->|is_not_found| D[true]
    ///     E[AggregateNotFound] -->|is_not_found| F[true]
    ///     G[ValidationError] -->|is_not_found| H[false]
    /// ```
    #[test]
    fn test_is_not_found() {
        // Should return true for not found errors
        assert!(DomainError::EntityNotFound {
            entity_type: "Test".to_string(),
            id: "123".to_string(),
        }
        .is_not_found());

        assert!(DomainError::AggregateNotFound("Test".to_string()).is_not_found());

        // Should return false for other errors
        assert!(!DomainError::ValidationError("Test".to_string()).is_not_found());
        assert!(!DomainError::Generic("Test".to_string()).is_not_found());
        assert!(!DomainError::ConcurrencyConflict {
            expected: 1,
            actual: 2
        }
        .is_not_found());
    }

    /// Test is_validation_error helper
    ///
    /// ```mermaid
    /// graph TD
    ///     A[ValidationError] -->|is_validation_error| B[true]
    ///     C[InvariantViolation] -->|is_validation_error| D[true]
    ///     E[BusinessRuleViolation] -->|is_validation_error| F[true]
    ///     G[EntityNotFound] -->|is_validation_error| H[false]
    /// ```
    #[test]
    fn test_is_validation_error() {
        // Should return true for validation-related errors
        assert!(DomainError::ValidationError("Test".to_string()).is_validation_error());
        assert!(DomainError::InvariantViolation("Test".to_string()).is_validation_error());
        assert!(DomainError::BusinessRuleViolation {
            rule: "Test".to_string()
        }
        .is_validation_error());

        // Should return false for other errors
        assert!(!DomainError::EntityNotFound {
            entity_type: "Test".to_string(),
            id: "123".to_string(),
        }
        .is_validation_error());
        assert!(!DomainError::Generic("Test".to_string()).is_validation_error());
        assert!(!DomainError::AuthorizationError("Test".to_string()).is_validation_error());
    }

    /// Test is_concurrency_error helper
    #[test]
    fn test_is_concurrency_error() {
        // Should return true for concurrency errors
        assert!(DomainError::ConcurrencyConflict {
            expected: 5,
            actual: 3,
        }
        .is_concurrency_error());

        // Should return false for other errors
        assert!(!DomainError::ValidationError("Test".to_string()).is_concurrency_error());
        assert!(!DomainError::EntityNotFound {
            entity_type: "Test".to_string(),
            id: "123".to_string(),
        }
        .is_concurrency_error());
        assert!(!DomainError::Generic("Test".to_string()).is_concurrency_error());
    }

    /// Test DomainResult type alias
    #[test]
    fn test_domain_result() {
        // Success case
        let success: DomainResult<i32> = Ok(42);
        assert!(success.is_ok());
        assert_eq!(success.ok().unwrap(), 42);

        // Error case
        let error: DomainResult<i32> = Err(DomainError::Generic("Failed".to_string()));
        assert!(error.is_err());
        assert_eq!(error.err().unwrap().to_string(), "Domain error: Failed");
    }

    /// Test error pattern matching
    #[test]
    fn test_error_pattern_matching() {
        let errors = vec![
            DomainError::EntityNotFound {
                entity_type: "User".to_string(),
                id: "123".to_string(),
            },
            DomainError::ConcurrencyConflict {
                expected: 1,
                actual: 2,
            },
        ];

        let mut entity_not_found_count = 0;
        let mut concurrency_count = 0;

        for error in errors {
            match error {
                DomainError::EntityNotFound { .. } => entity_not_found_count += 1,
                DomainError::ConcurrencyConflict { .. } => concurrency_count += 1,
                _ => {}
            }
        }

        assert_eq!(entity_not_found_count, 1);
        assert_eq!(concurrency_count, 1);
    }

    /// Test error conversion in functions
    #[test]
    fn test_error_in_functions() {
        fn may_fail(should_fail: bool) -> DomainResult<String> {
            if should_fail {
                Err(DomainError::ValidationError("Invalid input".to_string()))
            } else {
                Ok("Success".to_string())
            }
        }

        // Test success path
        let result = may_fail(false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");

        // Test error path
        let result = may_fail(true);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation_error());
    }

    /// Test error chaining with map_err
    #[test]
    fn test_error_chaining() {
        fn inner_operation() -> Result<i32, String> {
            Err("Inner error".to_string())
        }

        fn outer_operation() -> DomainResult<i32> {
            inner_operation().map_err(DomainError::InternalError)
        }

        let result = outer_operation();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Internal error: Inner error"
        );
    }

    /// Test untested error types
    #[test]
    fn test_untested_error_types() {
        // ComponentError removed in pure-library scope

        // NatsError removed in pure-library scope

        // Test NotFound (generic)
        let err = DomainError::NotFound("Resource XYZ".to_string());
        assert_eq!(err.to_string(), "Not found: Resource XYZ");
        assert!(!err.is_not_found()); // This is NOT included in is_not_found()

        // Test AlreadyExists (generic)
        let err = DomainError::AlreadyExists("Resource ABC".to_string());
        assert_eq!(err.to_string(), "Already exists: Resource ABC");

        // Test NotImplemented
        let err = DomainError::NotImplemented("Feature X".to_string());
        assert_eq!(err.to_string(), "Not implemented: Feature X");
    }

    /// Test serde_json error conversion
    #[test]
    fn test_serde_json_conversion() {
        // Create an invalid JSON to trigger serde_json error
        let invalid_json = "{ invalid json }";
        let serde_err = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();

        // Convert to DomainError
        let domain_err: DomainError = serde_err.into();

        // Check it's a SerializationError
        match domain_err {
            DomainError::SerializationError(msg) => {
                // Just verify it's a non-empty error message from serde_json
                assert!(!msg.is_empty());
                assert!(msg.contains("key") || msg.contains("expected") || msg.contains("invalid"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    /// Test error helper methods with NotFound variant
    #[test]
    fn test_is_not_found_edge_cases() {
        // The generic NotFound variant is NOT included in is_not_found()
        let generic_not_found = DomainError::NotFound("Something".to_string());
        assert!(!generic_not_found.is_not_found());

        // But specific not found errors are included
        let entity_not_found = DomainError::EntityNotFound {
            entity_type: "User".to_string(),
            id: "123".to_string(),
        };
        assert!(entity_not_found.is_not_found());

        // ComponentNotFound removed

        let aggregate_not_found = DomainError::AggregateNotFound("Agg".to_string());
        assert!(aggregate_not_found.is_not_found());
    }

    /// Test all error variants can be cloned
    #[test]
    fn test_all_errors_clone() {
        let errors: Vec<DomainError> = vec![
            // Component errors removed
            DomainError::EntityNotFound {
                entity_type: "Type".to_string(),
                id: "123".to_string(),
            },
            DomainError::InvalidOperation {
                reason: "test".to_string(),
            },
            DomainError::InvariantViolation("test".to_string()),
            DomainError::AggregateNotFound("test".to_string()),
            DomainError::InvalidStateTransition {
                from: "A".to_string(),
                to: "B".to_string(),
            },
            DomainError::ConcurrencyConflict {
                expected: 1,
                actual: 2,
            },
            DomainError::ValidationError("test".to_string()),
            DomainError::AuthorizationError("test".to_string()),
            DomainError::BusinessRuleViolation {
                rule: "test".to_string(),
            },
            DomainError::ContextBoundaryViolation("test".to_string()),
            DomainError::SerializationError("test".to_string()),
            DomainError::ExternalServiceError {
                service: "S".to_string(),
                message: "M".to_string(),
            },
            DomainError::Generic("test".to_string()),
            DomainError::InternalError("test".to_string()),
            DomainError::NotFound("test".to_string()),
            DomainError::AlreadyExists("test".to_string()),
            DomainError::NotImplemented("test".to_string()),
        ];

        for error in errors {
            let cloned = error.clone();
            assert_eq!(error.to_string(), cloned.to_string());
        }
    }

    /// Test error helper methods don't match incorrect variants
    #[test]
    fn test_helper_method_exclusivity() {
        let concurrency_err = DomainError::ConcurrencyConflict {
            expected: 1,
            actual: 2,
        };

        // Concurrency error should only match is_concurrency_error
        assert!(concurrency_err.is_concurrency_error());
        assert!(!concurrency_err.is_not_found());
        assert!(!concurrency_err.is_validation_error());

        let validation_err = DomainError::ValidationError("test".to_string());

        // Validation error should only match is_validation_error
        assert!(!validation_err.is_concurrency_error());
        assert!(!validation_err.is_not_found());
        assert!(validation_err.is_validation_error());

        let not_found_err = DomainError::EntityNotFound {
            entity_type: "User".to_string(),
            id: "123".to_string(),
        };

        // Not found error should only match is_not_found
        assert!(!not_found_err.is_concurrency_error());
        assert!(not_found_err.is_not_found());
        assert!(!not_found_err.is_validation_error());
    }
}
