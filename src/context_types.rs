// Copyright 2025 Cowboy AI, LLC.

//! Context types for bounded contexts and other domain boundaries

use serde::{Deserialize, Serialize};

/// Types of contexts that define boundaries in the domain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextType {
    /// A bounded context in DDD
    BoundedContext {
        /// Name of the bounded context
        name: String,
        /// Domain this context belongs to
        domain: String,
        /// Type of subdomain
        subdomain_type: SubdomainType,
    },

    /// An aggregate context (consistency boundary)
    AggregateContext {
        /// Name of the aggregate
        name: String,
        /// Type of aggregate
        aggregate_type: String,
    },

    /// A module context (functional grouping)
    ModuleContext {
        /// Name of the module
        name: String,
        /// Purpose of this module
        purpose: String,
    },

    /// A service context (capability boundary)
    ServiceContext {
        /// Name of the service
        name: String,
        /// Capability this service provides
        capability: String,
        /// Type of service
        service_type: ServiceType,
    },

    /// A team context (organizational boundary)
    TeamContext {
        /// Name of the team
        name: String,
        /// Team's responsibility
        responsibility: String,
    },

    /// A system context (technical boundary)
    SystemContext {
        /// Name of the system
        name: String,
        /// Type of system
        system_type: String,
    },

    /// A deployment context (runtime boundary)
    DeploymentContext {
        /// Name of the deployment
        name: String,
        /// Environment (dev, staging, prod)
        environment: String,
    },
}

/// Types of subdomains in DDD
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubdomainType {
    /// Core domain - the primary business differentiator
    Core,
    /// Supporting domain - necessary but not differentiating
    Supporting,
    /// Generic domain - common functionality
    Generic,
}

/// Types of services
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    /// Domain service - encapsulates domain logic
    Domain,
    /// Application service - orchestrates use cases
    Application,
    /// Infrastructure service - technical capabilities
    Infrastructure,
}

impl ContextType {
    /// Get the name of this context
    pub fn name(&self) -> &str {
        match self {
            ContextType::BoundedContext { name, .. } => name,
            ContextType::AggregateContext { name, .. } => name,
            ContextType::ModuleContext { name, .. } => name,
            ContextType::ServiceContext { name, .. } => name,
            ContextType::TeamContext { name, .. } => name,
            ContextType::SystemContext { name, .. } => name,
            ContextType::DeploymentContext { name, .. } => name,
        }
    }

    /// Check if this is a bounded context
    pub fn is_bounded_context(&self) -> bool {
        matches!(self, ContextType::BoundedContext { .. })
    }

    /// Check if this is an aggregate context
    pub fn is_aggregate_context(&self) -> bool {
        matches!(self, ContextType::AggregateContext { .. })
    }

    /// Check if this is a service context
    pub fn is_service_context(&self) -> bool {
        matches!(self, ContextType::ServiceContext { .. })
    }

    /// Get a human-readable type name
    pub fn type_name(&self) -> &str {
        match self {
            ContextType::BoundedContext { .. } => "Bounded Context",
            ContextType::AggregateContext { .. } => "Aggregate Context",
            ContextType::ModuleContext { .. } => "Module Context",
            ContextType::ServiceContext { .. } => "Service Context",
            ContextType::TeamContext { .. } => "Team Context",
            ContextType::SystemContext { .. } => "System Context",
            ContextType::DeploymentContext { .. } => "Deployment Context",
        }
    }
}

impl SubdomainType {
    /// Get a human-readable name
    pub fn display_name(&self) -> &str {
        match self {
            SubdomainType::Core => "Core Domain",
            SubdomainType::Supporting => "Supporting Domain",
            SubdomainType::Generic => "Generic Domain",
        }
    }

    /// Get the strategic importance level (1-3, higher is more important)
    pub fn importance_level(&self) -> u8 {
        match self {
            SubdomainType::Core => 3,
            SubdomainType::Supporting => 2,
            SubdomainType::Generic => 1,
        }
    }
}

impl ServiceType {
    /// Get a human-readable name
    pub fn display_name(&self) -> &str {
        match self {
            ServiceType::Domain => "Domain Service",
            ServiceType::Application => "Application Service",
            ServiceType::Infrastructure => "Infrastructure Service",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ContextType name extraction
    ///
    /// ```mermaid
    /// graph TD
    ///     A[ContextType] -->|name()| B[&str]
    ///     C[BoundedContext] -->|name| D["context_name"]
    ///     E[ServiceContext] -->|name| F["service_name"]
    ///     G[TeamContext] -->|name| H["team_name"]
    /// ```
    #[test]
    fn test_context_name() {
        let bounded = ContextType::BoundedContext {
            name: "OrderManagement".to_string(),
            domain: "Sales".to_string(),
            subdomain_type: SubdomainType::Core,
        };
        assert_eq!(bounded.name(), "OrderManagement");

        let aggregate = ContextType::AggregateContext {
            name: "Order".to_string(),
            aggregate_type: "Entity".to_string(),
        };
        assert_eq!(aggregate.name(), "Order");

        let module = ContextType::ModuleContext {
            name: "Pricing".to_string(),
            purpose: "Calculate prices".to_string(),
        };
        assert_eq!(module.name(), "Pricing");

        let service = ContextType::ServiceContext {
            name: "PaymentProcessor".to_string(),
            capability: "Process payments".to_string(),
            service_type: ServiceType::Domain,
        };
        assert_eq!(service.name(), "PaymentProcessor");

        let team = ContextType::TeamContext {
            name: "Platform".to_string(),
            responsibility: "Infrastructure".to_string(),
        };
        assert_eq!(team.name(), "Platform");

        let system = ContextType::SystemContext {
            name: "OrderSystem".to_string(),
            system_type: "Microservice".to_string(),
        };
        assert_eq!(system.name(), "OrderSystem");

        let deployment = ContextType::DeploymentContext {
            name: "prod-us-east".to_string(),
            environment: "production".to_string(),
        };
        assert_eq!(deployment.name(), "prod-us-east");
    }

    /// Test context type classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[ContextType] -->|is_bounded_context| B{Check Type}
    ///     B -->|BoundedContext| C[true]
    ///     B -->|Other| D[false]
    /// ```
    #[test]
    fn test_context_type_classification() {
        let bounded = ContextType::BoundedContext {
            name: "Test".to_string(),
            domain: "Domain".to_string(),
            subdomain_type: SubdomainType::Core,
        };
        assert!(bounded.is_bounded_context());
        assert!(!bounded.is_aggregate_context());
        assert!(!bounded.is_service_context());

        let aggregate = ContextType::AggregateContext {
            name: "Test".to_string(),
            aggregate_type: "Entity".to_string(),
        };
        assert!(!aggregate.is_bounded_context());
        assert!(aggregate.is_aggregate_context());
        assert!(!aggregate.is_service_context());

        let service = ContextType::ServiceContext {
            name: "Test".to_string(),
            capability: "Capability".to_string(),
            service_type: ServiceType::Domain,
        };
        assert!(!service.is_bounded_context());
        assert!(!service.is_aggregate_context());
        assert!(service.is_service_context());
    }

    /// Test context type names
    #[test]
    fn test_context_type_name() {
        assert_eq!(
            ContextType::BoundedContext {
                name: "Test".to_string(),
                domain: "Domain".to_string(),
                subdomain_type: SubdomainType::Core,
            }
            .type_name(),
            "Bounded Context"
        );

        assert_eq!(
            ContextType::AggregateContext {
                name: "Test".to_string(),
                aggregate_type: "Entity".to_string(),
            }
            .type_name(),
            "Aggregate Context"
        );

        assert_eq!(
            ContextType::ModuleContext {
                name: "Test".to_string(),
                purpose: "Purpose".to_string(),
            }
            .type_name(),
            "Module Context"
        );

        assert_eq!(
            ContextType::ServiceContext {
                name: "Test".to_string(),
                capability: "Cap".to_string(),
                service_type: ServiceType::Domain,
            }
            .type_name(),
            "Service Context"
        );

        assert_eq!(
            ContextType::TeamContext {
                name: "Test".to_string(),
                responsibility: "Resp".to_string(),
            }
            .type_name(),
            "Team Context"
        );

        assert_eq!(
            ContextType::SystemContext {
                name: "Test".to_string(),
                system_type: "Type".to_string(),
            }
            .type_name(),
            "System Context"
        );

        assert_eq!(
            ContextType::DeploymentContext {
                name: "Test".to_string(),
                environment: "prod".to_string(),
            }
            .type_name(),
            "Deployment Context"
        );
    }

    /// Test SubdomainType display names and importance
    ///
    /// ```mermaid
    /// graph TD
    ///     A[SubdomainType] -->|importance_level| B[u8]
    ///     C[Core] -->|3| D[High]
    ///     E[Supporting] -->|2| F[Medium]
    ///     G[Generic] -->|1| H[Low]
    /// ```
    #[test]
    fn test_subdomain_type() {
        // Display names
        assert_eq!(SubdomainType::Core.display_name(), "Core Domain");
        assert_eq!(
            SubdomainType::Supporting.display_name(),
            "Supporting Domain"
        );
        assert_eq!(SubdomainType::Generic.display_name(), "Generic Domain");

        // Importance levels
        assert_eq!(SubdomainType::Core.importance_level(), 3);
        assert_eq!(SubdomainType::Supporting.importance_level(), 2);
        assert_eq!(SubdomainType::Generic.importance_level(), 1);

        // Verify ordering
        assert!(
            SubdomainType::Core.importance_level() > SubdomainType::Supporting.importance_level()
        );
        assert!(
            SubdomainType::Supporting.importance_level()
                > SubdomainType::Generic.importance_level()
        );
    }

    /// Test ServiceType display names
    #[test]
    fn test_service_type() {
        assert_eq!(ServiceType::Domain.display_name(), "Domain Service");
        assert_eq!(
            ServiceType::Application.display_name(),
            "Application Service"
        );
        assert_eq!(
            ServiceType::Infrastructure.display_name(),
            "Infrastructure Service"
        );
    }

    /// Test serialization and deserialization
    #[test]
    fn test_serde() {
        // Test ContextType variants
        let contexts = vec![
            ContextType::BoundedContext {
                name: "Sales".to_string(),
                domain: "Commerce".to_string(),
                subdomain_type: SubdomainType::Core,
            },
            ContextType::AggregateContext {
                name: "Order".to_string(),
                aggregate_type: "Root".to_string(),
            },
            ContextType::ServiceContext {
                name: "PaymentService".to_string(),
                capability: "Process payments".to_string(),
                service_type: ServiceType::Domain,
            },
        ];

        for context in contexts {
            let json = serde_json::to_string(&context).unwrap();
            let deserialized: ContextType = serde_json::from_str(&json).unwrap();
            assert_eq!(context, deserialized);
        }

        // Test SubdomainType
        let subdomains = vec![
            SubdomainType::Core,
            SubdomainType::Supporting,
            SubdomainType::Generic,
        ];

        for subdomain in subdomains {
            let json = serde_json::to_string(&subdomain).unwrap();
            let deserialized: SubdomainType = serde_json::from_str(&json).unwrap();
            assert_eq!(subdomain, deserialized);
        }

        // Test ServiceType
        let services = vec![
            ServiceType::Domain,
            ServiceType::Application,
            ServiceType::Infrastructure,
        ];

        for service in services {
            let json = serde_json::to_string(&service).unwrap();
            let deserialized: ServiceType = serde_json::from_str(&json).unwrap();
            assert_eq!(service, deserialized);
        }
    }

    /// Test complex bounded context creation
    #[test]
    fn test_bounded_context_creation() {
        let context = ContextType::BoundedContext {
            name: "InventoryManagement".to_string(),
            domain: "Warehouse".to_string(),
            subdomain_type: SubdomainType::Supporting,
        };

        assert_eq!(context.name(), "InventoryManagement");
        assert!(context.is_bounded_context());
        assert_eq!(context.type_name(), "Bounded Context");

        // Verify subdomain type is preserved
        if let ContextType::BoundedContext { subdomain_type, .. } = &context {
            assert_eq!(subdomain_type.importance_level(), 2);
            assert_eq!(subdomain_type.display_name(), "Supporting Domain");
        }
    }

    /// Test service context with different service types
    #[test]
    fn test_service_context_types() {
        let domain_service = ContextType::ServiceContext {
            name: "PricingService".to_string(),
            capability: "Calculate prices".to_string(),
            service_type: ServiceType::Domain,
        };

        let app_service = ContextType::ServiceContext {
            name: "OrderOrchestrator".to_string(),
            capability: "Orchestrate order flow".to_string(),
            service_type: ServiceType::Application,
        };

        let infra_service = ContextType::ServiceContext {
            name: "EmailService".to_string(),
            capability: "Send emails".to_string(),
            service_type: ServiceType::Infrastructure,
        };

        // All should be service contexts
        assert!(domain_service.is_service_context());
        assert!(app_service.is_service_context());
        assert!(infra_service.is_service_context());

        // But not other types
        assert!(!domain_service.is_bounded_context());
        assert!(!app_service.is_aggregate_context());
    }

    /// Test equality and hashing
    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();

        // Add different context types
        set.insert(ContextType::BoundedContext {
            name: "Sales".to_string(),
            domain: "Commerce".to_string(),
            subdomain_type: SubdomainType::Core,
        });

        set.insert(ContextType::TeamContext {
            name: "Platform".to_string(),
            responsibility: "Infrastructure".to_string(),
        });

        assert_eq!(set.len(), 2);

        // Same context should not increase size
        set.insert(ContextType::BoundedContext {
            name: "Sales".to_string(),
            domain: "Commerce".to_string(),
            subdomain_type: SubdomainType::Core,
        });

        assert_eq!(set.len(), 2);

        // Different name should increase size
        set.insert(ContextType::BoundedContext {
            name: "Inventory".to_string(),
            domain: "Commerce".to_string(),
            subdomain_type: SubdomainType::Core,
        });

        assert_eq!(set.len(), 3);
    }

    /// Test all context types have consistent behavior
    #[test]
    fn test_all_context_types_consistency() {
        let contexts = vec![
            ContextType::BoundedContext {
                name: "Test1".to_string(),
                domain: "Domain".to_string(),
                subdomain_type: SubdomainType::Core,
            },
            ContextType::AggregateContext {
                name: "Test2".to_string(),
                aggregate_type: "Type".to_string(),
            },
            ContextType::ModuleContext {
                name: "Test3".to_string(),
                purpose: "Purpose".to_string(),
            },
            ContextType::ServiceContext {
                name: "Test4".to_string(),
                capability: "Cap".to_string(),
                service_type: ServiceType::Domain,
            },
            ContextType::TeamContext {
                name: "Test5".to_string(),
                responsibility: "Resp".to_string(),
            },
            ContextType::SystemContext {
                name: "Test6".to_string(),
                system_type: "Type".to_string(),
            },
            ContextType::DeploymentContext {
                name: "Test7".to_string(),
                environment: "env".to_string(),
            },
        ];

        // All should have non-empty names and type names
        for context in &contexts {
            assert!(!context.name().is_empty());
            assert!(!context.type_name().is_empty());
        }

        // Classification should be mutually exclusive
        let bounded_count = contexts.iter().filter(|c| c.is_bounded_context()).count();
        let aggregate_count = contexts.iter().filter(|c| c.is_aggregate_context()).count();
        let service_count = contexts.iter().filter(|c| c.is_service_context()).count();

        assert_eq!(bounded_count, 1);
        assert_eq!(aggregate_count, 1);
        assert_eq!(service_count, 1);
    }
}
