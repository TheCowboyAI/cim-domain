# CIM Domain Examples

This directory contains comprehensive examples demonstrating the CIM Domain framework through realistic user stories. Each example focuses on a different stakeholder's perspective and showcases specific framework capabilities.

## User Story Examples

### 1. Component Developer - Building a Reusable UI Component
**File:** `user_story_1_component_developer.rs`

Demonstrates how to build reusable components that:
- Maintain their own state
- Emit domain events for all interactions
- Can be embedded in different contexts
- Provide features like search suggestions

**Run:** `cargo run --example user_story_1_component_developer`

### 2. System Architect - Defining Domain Boundaries
**File:** `user_story_2_system_architect.rs`

Shows how to:
- Define clear domain boundaries
- Establish cross-domain communication rules
- Implement saga patterns for distributed transactions
- Enforce domain invariants

**Run:** `cargo run --example user_story_2_system_architect`

### 3. Event Stream Manager - Setting up Event Flows
**File:** `user_story_3_event_stream_manager.rs`

Illustrates:
- Event stream configuration with retention policies
- Consumer setup with delivery policies
- Event routing with pattern matching
- Health monitoring and metrics
- Event transformation and enrichment

**Run:** `cargo run --example user_story_3_event_stream_manager`

### 4. Data Analyst - Building Projections and Queries
**File:** `user_story_4_data_analyst.rs`

Demonstrates:
- Creating projections from event streams
- Building complex queries with filters
- Calculating aggregations
- Generating analytics reports
- Real-time data analysis

**Run:** `cargo run --example user_story_4_data_analyst`

### 5. Integration Engineer - Cross-domain Communication
**File:** `user_story_5_integration_engineer.rs`

Shows how to:
- Set up event bridges between domains
- Implement API gateways
- Handle message transformation
- Use circuit breakers for resilience
- Manage retry queues

**Run:** `cargo run --example user_story_5_integration_engineer`

## Comprehensive Demo

**File:** `comprehensive_demo.rs`

This demo combines all five user stories to create a complete e-commerce platform. It demonstrates:
- How all components work together
- A complete customer journey from search to fulfillment
- Domain boundaries in action
- Event flow through the entire system
- Real-time analytics and monitoring

**Run:** `cargo run --example comprehensive_demo`

## Key Concepts Demonstrated

### Domain-Driven Design
- Bounded contexts with clear boundaries
- Ubiquitous language per domain
- Domain events as the primary communication mechanism
- Aggregate roots and entities

### Event Sourcing
- All state changes captured as events
- Event streams with configurable retention
- Projections built from event history
- Event replay capabilities

### CQRS (Command Query Responsibility Segregation)
- Separate write models (commands) and read models (queries)
- Optimized projections for different query patterns
- Eventually consistent read models

### Resilience Patterns
- Circuit breakers for fault tolerance
- Retry queues with exponential backoff
- Health monitoring and alerting
- Graceful degradation

### Integration Patterns
- Event bridge for loose coupling
- API gateway for external access
- Message transformation
- Content enrichment

## Running the Examples

1. Ensure you have Rust installed (1.70 or later)
2. Clone the repository
3. Run individual examples:
   ```bash
   cargo run --example user_story_1_component_developer
   cargo run --example user_story_2_system_architect
   cargo run --example user_story_3_event_stream_manager
   cargo run --example user_story_4_data_analyst
   cargo run --example user_story_5_integration_engineer
   cargo run --example comprehensive_demo
   ```

## Learning Path

For the best learning experience, we recommend running the examples in order:

1. Start with the Component Developer example to understand basic component design
2. Move to System Architect to see how domains are structured
3. Explore Event Stream Manager to understand event flow
4. Try Data Analyst to see how to build analytics
5. Learn Integration patterns with the Integration Engineer example
6. Finally, run the Comprehensive Demo to see everything working together

Each example is self-contained and includes inline documentation explaining the concepts being demonstrated.