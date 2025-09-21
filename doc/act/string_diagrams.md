# CIM Domain — String and Commutative Diagrams

This document provides categorical string/commutative diagrams for the current domain and records the laws/conditions under which they commute. SVG diagrams are stored under `doc/act/diagrams/` and referenced from `domain-graph.json`. If a diagram fails to commute, the conditions and required modifications are stated.

## DDD Core: cim-domain as a Category

See: doc/act/diagrams/ddd_domain_category.svg

- Objects: Command, Aggregate, DomainEvent, StateMachine
- Morphisms: handle (Kleisli Result), decide→events (transformation), fold(events) (free monoid fold)
- Commutation: fold ∘ decide ∘ handle = transition; fold is associative with [] as identity
 - Convention: “fold” means a left fold over time-ordered events (oldest → newest) unless explicitly stated as a “right fold”. Implementation: `state = events.iter().fold(init, |s, e| apply(s, e))`.

## Aggregate Composition

See: doc/act/diagrams/aggregate_composition.svg

- Aggregates encapsulate identity (EntityId<T>), value objects (invariants), and state machines
- Commands decide events; events fold back to state; invariants form boundaries

## Event Sourcing Fold

See: doc/act/diagrams/event_sourcing_fold.svg

- Events form a free monoid under concatenation; fold is a monoid homomorphism
- Laws: fold(x) ∘ fold(y) = fold(x ∘ y); identity=[]

## CQRS Projection Functor

See: doc/act/diagrams/cqrs_projection_functor.svg

- Projection P maps events to read model updates; functoriality P(e2 ∘ e1) = P(e2) ∘ P(e1)
- Queries read from ReadModel producing QueryResponse
- Regression: `tests/query_read_path_tests.rs` proves projections handle/clear events and query handlers respond using the in-memory read model; unit tests in `src/projections/mod.rs` cover the projection trait directly.

## Causation/Correlation Commutation

See: doc/act/diagrams/causation_correlation_commutation.svg

- Correlation preserved across causation chain; causation set to prior message_id
- Verified in `tests/envelope_identity_tests.rs` (root → follow-up command chain)

## Identity Envelope (Command → Event)

See: doc/act/diagrams/identity_envelope_v2.dot.svg

- Root envelopes created with `CommandEnvelope::new(_)/new_in_tx` establish the correlation/cause pair carried through follow-up commands and queries.
- `DomainEventEnvelope::inline` mirrors the identity, capturing `event_id`, `aggregate_id`, correlation, and causation before optionally swapping payloads for a CID.
- Event IDs (`EventId::new`) are UUID v7, providing a monotone timestamp surface for downstream ordering proofs.
- Ack/response artifacts (`CommandAcknowledgment`, `QueryAcknowledgment`, `QueryResponse`) are checked to reflect the originating envelope identity before emitting downstream projections.
- Regression: `tests/envelope_identity_tests.rs` exercises the entire chain (root command → follow-up command/query → acknowledgments → event envelope) and the inline→CID transition.
## Content Addressing (DomainNode)

See: doc/act/diagrams/addressing_v2.dot.svg

- `DomainNode::from_payload` captures payload metadata, codec, and content type, returning both a metadata envelope and a root `DomainCid` scoped to `domain-node`.
- CID chains (`CidChain::new/verify_chain`) thread previous/current pairs to preserve causality.
- Regression: `tests/cid_content_addressing_tests.rs` asserts that DomainCID/DomainNode metadata is stable, chain verification works, and `generate_cid` hashes serialized payloads.

## Content Addressing Buckets & Index

See: doc/act/diagrams/content_addressing_buckets_v2.dot.svg

- `BucketLog::append` records an append-only tail of `BucketEntry` values, each capturing the new `DomainCid`, the prior tail, and a bucket-local sequence number.
- `CidIndexEntry::new` mirrors the bucket append, storing the current bucket identifier, subject hint, and payload parent while leaving a trail of `MoveHistoryEntry` rows keyed by the originating `EventId`.
- Regression: `tests/cid_content_addressing_tests.rs` covers sequence growth (`BucketLog`), move bookkeeping (`CidIndexEntry::record_move`), and ensures the index mirrors the append semantics implemented in `src/object_store.rs`.

## Subject Algebra (Free Monoid)

See: doc/act/diagrams/subject_algebra_v2.dot.svg

- `Subject` is the free monoid over validated `SubjectSegment` tokens with concatenation as the operation and `Subject::root()` as the identity.
- `SubjectPattern` extends the algebra with `*` (single segment) and a terminal `>` (multi-segment) wildcard captured via `SubjectPatternSegment`.
- Law: `concat(concat(s1, s2), s3) = concat(s1, concat(s2, s3))` and `concat(root, s) = s = concat(s, root)`.
- Regression: `tests/subject_algebra_tests.rs` exercises associativity/identity, wildcard semantics, and validation; unit tests in `src/subject.rs` cover the internal constructors.

## Domain Path Algebra (Hierarchical Namespace)

See: doc/act/diagrams/domain_path_algebra_v2.dot.svg

- `DomainPath` models the canonical `cim.domain.<bounded_context>.<facet>.<name>` namespace; `DomainPath::root()` (`cim.domain`) acts as the monoid identity and concatenation composes additional facets.
- `DomainPathSegment` captures validated dotted tokens; `DomainArtifactKind` annotates known facets (command, aggregate, value, etc.) rendered as path segments.
- Law: `concat(concat(p_bc, p_facet), p_name) = concat(p_bc, concat(p_facet, p_name))` with `root` as the two-sided identity.
- Regression: `tests/domain_path_algebra_tests.rs` validates prefix enforcement, helpers (`command`, `value`), and monoid laws; module tests in `src/domain_path.rs` exercise accessors and builders.

## Domain Algebra Overview (Path ⇄ Subject ⇄ Persistence)

See: doc/act/diagrams/domain_algebra_overview_v2.dot.svg

- `DomainPath` introduces the canonical naming hierarchy, `Subject` captures routing algebra, and the content-addressing objects (`BucketLog`, `CidIndexEntry`, `DomainCid`) show how persisted state is addressed.
- Morphisms reuse the existing diagrams: domain paths compose from segments, subjects form a free monoid, and persistence indices map CIDs into buckets with `EventId`-tracked move history.
- Regression: union of `tests/subject_algebra_tests.rs`, `tests/domain_path_algebra_tests.rs`, and `tests/cid_content_addressing_tests.rs` keeps each algebra slice and their intersections honest.

## Aggregate, Entity & Value Composition

See: doc/act/diagrams/aggregate_entity_value_v2.dot.svg

- `AggregateRoot` contains domain entities, is governed by state machines, constrained by policies, and emits events; policies define the `DomainInvariant`s an aggregate must enforce.
- `DomainEntity` owns `ValueCollection`s, which in turn contain immutable `ValueObject`s representing state; invariant violations surface as structured `InvariantViolation` value objects.
- Regression: entity/ID semantics are doc-tested in `src/entity.rs`, value object collections are proved in `tests/value_collection_monoid_tests.rs`, and invariant workflows are exercised in `tests/domain_invariant_tests.rs`.

## Saga as Composed Aggregate

See: doc/act/diagrams/saga_as_composed_aggregate.svg

- Product of aggregate categories lifted into a Saga; compensations as natural transformations

## Bounded Context Lifting

See: doc/act/diagrams/bounded_context_functors.svg

- Bounded Contexts are categories; lift functor F: BC₁ → BC₂ preserves id and composition

## Ontology → Quality Dimensions

See: doc/act/diagrams/ontology_to_qd.svg

- Ontology objects (Concept nodes + typed relations) map to Quality Vectors via a qualifier: Qualify: (Ontology × Concept, Schema) → QualityVector
- Laws: Qualify respects ontology morphisms (refinements do not break consistency); monotonic under concept refinement for monotone features; functorial in Schema transformations if linear mappings exist.

## Entity Naming

See: doc/act/diagrams/entity_naming.svg

- Given: entity features under a QualitySchema and a set of concept prototypes (Concept → QualityVector)
- Vectorize features (vector_from_features), then select top concept(s) by similarity (suggest_by_prototypes)
- UL integration: HasConcept on types + named concepts in ontology
## Why a Topos in the Domain

See: doc/act/diagrams/topos_overview.svg

- A topos equips the domain category with finite limits, exponentials, and a subobject classifier Ω.
- Internal logic (Heyting algebra) lets us express invariants, policies, and guards as predicates and reason compositionally.
- Practically: we model guard conditions and valid-state subsets as subobjects; commands become domain-restricted via pullbacks; policies compose via ∧, ∨, ⇒.

### Subobject Classifier and Guards

See: doc/act/diagrams/subobject_classifier.svg and doc/act/diagrams/guarded_command_pullback.svg

- Every mono m: A ↪ X has a characteristic map χ_m: X → Ω classifying membership.
- Naturality: for f: Y → X, χ_{f* m} = χ_m ∘ f (pull back predicates along morphisms).
  - Guarded commands: restrict handle to S_allowed ⊆ Aggregate via pullback along χ_guard, making partial operations total on the subobject (modeled as Kleisli in code).

## Value Collections (Shape-Agnostic Monoid)

See: doc/act/diagrams/value_collection_monoid.svg

- ValueCollection abstracts “values in an entity” without fixing the data structure.
- Shape options and operations:
  - Sequence/Array: operation = concat; identity = []
  - Set: operation = union; identity = ∅
  - Bag/Multiset: operation = multiset-union; identity = ∅
- Laws: In all cases, (ValueCollection, ⊕, ∅) forms a monoid (associative; identity element).
- Diagram coverage: concat_collections morphism documents the chosen ⊕. Tests validate associativity and identity for Vec (concat) and BTreeSet (union); regression coverage lives in `tests/value_collection_monoid_tests.rs`.

## Concept Graphs: CIM vs Domain

See: doc/act/diagrams/concept_graphs.svg

- Two graphs, not layers or a fixed hierarchy:
  - CIM Core Concepts: the immutable 10 cognitive concepts form a graph (relations live in the CIM space).
  - Domain Concepts: DDD/ECS/CQRS/EventSourcing primitives form their own graph within the Domain.
- UL mapping relates nodes across graphs: we project Domain concepts/objects to subsets of CIM core concepts. This is computed by the UL projection tool and guided by classifier heuristics.
- Policy: we do not redefine CIM core in the Domain graph; we reference the core and maintain cross-graph mappings. Relationships are arbitrary (graph), not a tree.

Note on events: event relationships naturally form a hypergraph (an event can relate multiple participants). Our diagrams abstract this by edges and annotations; the underlying model remains graph/hypergraph, not layered.
