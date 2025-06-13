# Aggregates as Enriched Categories and Topos Composition

## Overview

In our domain model, DDD Aggregates are not just simple objects - they are **Enriched Categories** that become instantiated through dependency injection. When multiple aggregates compose together, they form a **Topos** with internal logic and comprehension principles.

## Aggregates as Enriched Categories

### The Enrichment Structure

Each aggregate is enriched over a monoidal category of **State Transitions** and **Domain Events**:

```rust
/// An Aggregate is a category enriched over the monoidal category of state transitions
pub struct AggregateAsEnrichedCategory<A: AggregateRoot> {
    /// Objects are the possible states of the aggregate
    pub states: HashSet<A::State>,

    /// Morphisms are state transitions (enriched with events)
    pub transitions: EnrichedTransitions<A>,

    /// The enrichment captures:
    /// - Cost: Number of events generated
    /// - Distance: Semantic distance between states
    /// - Time: Expected transition duration
    /// - Probability: Likelihood of transition success
    pub enrichment: AggregateEnrichment,
}

pub struct AggregateEnrichment {
    /// Event cost of transitions
    pub event_cost: Box<dyn Fn(&StateTransition) -> usize>,

    /// Semantic distance between states
    pub state_distance: Box<dyn Fn(&State, &State) -> f64>,

    /// Time cost of transitions
    pub time_cost: Box<dyn Fn(&StateTransition) -> Duration>,

    /// Business value of states
    pub state_value: Box<dyn Fn(&State) -> f64>,
}
```

### Example: Order Aggregate as Enriched Category

```rust
impl EnrichedCategory for OrderAggregate {
    type Object = OrderState;
    type Morphism = OrderStateTransition;
    type Enrichment = OrderEnrichment;

    fn enrichment() -> Self::Enrichment {
        OrderEnrichment {
            // Event cost: how many events does this transition generate?
            event_cost: |transition| match transition {
                OrderStateTransition::Submit => 3, // OrderSubmitted, InventoryReserved, PaymentRequested
                OrderStateTransition::Cancel => 2, // OrderCancelled, InventoryReleased
                OrderStateTransition::Ship => 4,   // OrderShipped, TrackingCreated, CustomerNotified, InventoryUpdated
                _ => 1,
            },

            // Semantic distance: how "far" are these states?
            state_distance: |from, to| match (from, to) {
                (Draft, Submitted) => 1.0,      // Adjacent states
                (Draft, Delivered) => 5.0,      // Many steps apart
                (Cancelled, Delivered) => f64::INFINITY, // Impossible transition
                _ => calculate_path_length(from, to),
            },

            // Business value of states
            state_value: |state| match state {
                Delivered => 100.0,  // Maximum value - order completed
                Shipped => 80.0,     // High value - commitment made
                PaymentConfirmed => 60.0,
                Cancelled => -10.0,  // Negative value - lost opportunity
                _ => 0.0,
            },
        }
    }
}
```

### Dependency Injection Instantiation

Aggregates become enriched categories through dependency injection:

```rust
/// Dependency injection container that enriches aggregates
pub struct AggregateContainer {
    /// Services that provide enrichment
    pub event_store: Arc<EventStore>,
    pub metrics_collector: Arc<MetricsCollector>,
    pub business_rules: Arc<BusinessRules>,
    pub semantic_analyzer: Arc<SemanticAnalyzer>,
}

impl AggregateContainer {
    /// Instantiate an aggregate as an enriched category
    pub fn instantiate<A: AggregateRoot>(&self, id: A::Id) -> EnrichedAggregate<A> {
        EnrichedAggregate {
            aggregate: A::new(id),

            // Inject enrichment from services
            enrichment: AggregateEnrichment {
                event_cost: {
                    let store = self.event_store.clone();
                    Box::new(move |transition| {
                        store.count_events_for_transition(transition)
                    })
                },

                state_distance: {
                    let analyzer = self.semantic_analyzer.clone();
                    Box::new(move |from, to| {
                        analyzer.semantic_distance(from, to)
                    })
                },

                time_cost: {
                    let metrics = self.metrics_collector.clone();
                    Box::new(move |transition| {
                        metrics.average_transition_time(transition)
                    })
                },

                state_value: {
                    let rules = self.business_rules.clone();
                    Box::new(move |state| {
                        rules.calculate_business_value(state)
                    })
                },
            },
        }
    }
}
```

## Aggregate Composition in a Topos

When multiple aggregates compose together, they form a **Topos** - a category with internal logic:

### The Aggregate Topos Structure

```rust
/// Multiple aggregates compose into a topos
pub struct AggregateTopos {
    /// The aggregates are objects in the topos
    pub aggregates: HashMap<AggregateId, Box<dyn EnrichedAggregate>>,

    /// Relationships between aggregates are morphisms
    pub relationships: Graph<AggregateId, AggregateRelationship>,

    /// The subobject classifier (truth values for aggregate predicates)
    pub truth_object: AggregatePredicateSpace,

    /// Internal logic for reasoning about aggregates
    pub logic: AggregateLogic,
}

pub struct AggregateLogic {
    /// Invariants that must hold across aggregates
    pub invariants: Vec<CrossAggregateInvariant>,

    /// Saga orchestration logic
    pub sagas: HashMap<SagaId, SagaDefinition>,

    /// Comprehension: create sub-aggregates from predicates
    pub comprehension: ComprehensionEngine,
}
```

### Example: Order-Inventory-Payment Topos

```rust
/// A topos composing Order, Inventory, and Payment aggregates
pub struct OrderFulfillmentTopos {
    pub order: EnrichedAggregate<Order>,
    pub inventory: EnrichedAggregate<Inventory>,
    pub payment: EnrichedAggregate<Payment>,
    pub topos: AggregateTopos,
}

impl OrderFulfillmentTopos {
    pub fn new(container: &AggregateContainer) -> Self {
        let mut topos = AggregateTopos::new();

        // Add aggregates as objects
        let order = container.instantiate::<Order>(order_id);
        let inventory = container.instantiate::<Inventory>(inventory_id);
        let payment = container.instantiate::<Payment>(payment_id);

        topos.add_aggregate(order);
        topos.add_aggregate(inventory);
        topos.add_aggregate(payment);

        // Define relationships (morphisms)
        topos.add_relationship(
            order.id(),
            inventory.id(),
            AggregateRelationship::References {
                field: "items",
                cardinality: OneToMany,
            }
        );

        topos.add_relationship(
            order.id(),
            payment.id(),
            AggregateRelationship::Owns {
                field: "payment",
                cascade: true,
            }
        );

        // Define cross-aggregate invariants
        topos.add_invariant(CrossAggregateInvariant {
            name: "OrderPaymentConsistency",
            predicate: |order, payment| {
                order.total_amount() == payment.amount()
            },
        });

        topos.add_invariant(CrossAggregateInvariant {
            name: "InventoryAvailability",
            predicate: |order, inventory| {
                order.items().all(|item| {
                    inventory.available_quantity(item.sku) >= item.quantity
                })
            },
        });

        Self { order, inventory, payment, topos }
    }
}
```

### Topos Operations

The topos provides powerful operations for reasoning about aggregate compositions:

```rust
impl AggregateTopos {
    /// Comprehension: Create a sub-aggregate from a predicate
    pub fn comprehend<P: AggregatePredicate>(
        &self,
        predicate: P,
    ) -> SubAggregate {
        // { a ∈ Aggregates | predicate(a) }
        let matching = self.aggregates.values()
            .filter(|agg| predicate.evaluate(agg))
            .collect();

        SubAggregate::new(matching, predicate)
    }

    /// Check if an invariant holds across all aggregates
    pub fn verify_invariant(&self, invariant: &CrossAggregateInvariant) -> bool {
        self.logic.invariants.contains(invariant) &&
        invariant.check_all(&self.aggregates)
    }

    /// Execute a saga across multiple aggregates
    pub fn execute_saga(&mut self, saga: SagaDefinition) -> Result<SagaResult> {
        let mut saga_executor = SagaExecutor::new(&mut self.aggregates);

        // Each saga step is a morphism in the topos
        for step in saga.steps {
            match saga_executor.execute_step(step) {
                Ok(events) => saga_executor.record_events(events),
                Err(e) => {
                    saga_executor.compensate()?;
                    return Err(e);
                }
            }
        }

        Ok(saga_executor.complete())
    }
}
```

### Internal Logic and Comprehension

The topos provides an internal logic for reasoning about aggregates:

```rust
/// Predicate logic for aggregates
pub enum AggregatePredicate {
    /// Basic predicates
    InState { aggregate: AggregateId, state: State },
    HasValue { aggregate: AggregateId, field: String, value: Value },

    /// Logical combinations
    And(Box<AggregatePredicate>, Box<AggregatePredicate>),
    Or(Box<AggregatePredicate>, Box<AggregatePredicate>),
    Not(Box<AggregatePredicate>),
    Implies(Box<AggregatePredicate>, Box<AggregatePredicate>),

    /// Quantifiers over aggregates
    ForAll {
        variable: AggregateId,
        aggregate_type: AggregateType,
        predicate: Box<AggregatePredicate>,
    },
    Exists {
        variable: AggregateId,
        aggregate_type: AggregateType,
        predicate: Box<AggregatePredicate>,
    },
}

/// Example: Complex business rule as a predicate
let can_fulfill_order = AggregatePredicate::And(
    Box::new(AggregatePredicate::InState {
        aggregate: order_id,
        state: OrderState::PaymentConfirmed,
    }),
    Box::new(AggregatePredicate::ForAll {
        variable: item_id,
        aggregate_type: AggregateType::OrderItem,
        predicate: Box::new(AggregatePredicate::Exists {
            variable: inventory_id,
            aggregate_type: AggregateType::Inventory,
            predicate: Box::new(AggregatePredicate::HasValue {
                aggregate: inventory_id,
                field: "available_quantity",
                value: Value::GreaterThan(item_quantity),
            }),
        }),
    }),
);
```

## Practical Benefits

### 1. **Optimal State Transitions**
The enrichment structure allows finding optimal paths through aggregate states:

```rust
let optimal_path = order_aggregate
    .find_optimal_path(
        OrderState::Draft,
        OrderState::Delivered,
        OptimizationCriteria::MinimizeEventCost,
    );
```

### 2. **Semantic Similarity**
Find similar states or transitions based on enriched distance:

```rust
let similar_states = order_aggregate
    .find_similar_states(
        OrderState::PaymentPending,
        max_distance: 2.0,
    );
```

### 3. **Cross-Aggregate Consistency**
The topos ensures invariants hold across aggregate boundaries:

```rust
let consistency_check = order_fulfillment_topos
    .verify_all_invariants()
    .map_err(|violations| {
        // Handle consistency violations
    });
```

### 4. **Saga Orchestration**
Complex workflows are morphisms in the topos:

```rust
let order_fulfillment_saga = SagaDefinition {
    name: "FulfillOrder",
    steps: vec![
        SagaStep::UpdateAggregate(order_id, OrderCommand::ConfirmPayment),
        SagaStep::UpdateAggregate(inventory_id, InventoryCommand::ReserveItems),
        SagaStep::UpdateAggregate(order_id, OrderCommand::MarkReadyToShip),
    ],
    compensations: vec![
        // Compensation steps in reverse order
    ],
};

let result = order_fulfillment_topos.execute_saga(order_fulfillment_saga)?;
```

## Implementation in CIM

In our implementation:

1. **State Machines** (Moore/Mealy) provide the morphisms in the enriched category
2. **Event Outputs** provide the enrichment (cost = number of events)
3. **Aggregate Composition** happens through the topos structure
4. **Dependency Injection** provides the runtime enrichment values

This mathematical foundation gives us:
- **Principled Composition**: Aggregates compose according to topos laws
- **Optimal Execution**: Find best paths through state spaces
- **Consistency Guarantees**: Invariants enforced by topos logic
- **Semantic Understanding**: Enrichment captures business meaning

The beauty is that this isn't just theory - it's directly implementable and provides real business value through better aggregate design and composition.
