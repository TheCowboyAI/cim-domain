# Domain Objects Catalog (UL‑Aligned)

This catalog enumerates the union of Domain Objects defined in code and in the category lens (domain‑graph.json), with a short purpose statement and alignment to the Ubiquitous Language (UL). UL concept ids follow the projection slug rule (lowercased name; spaces/specials → '_'). Items marked "external" are references to other domains. Items marked "conceptual" denote lens‑level constructs without a concrete core type.

## Aggregates & Entities
- AggregateRoot — Consistency boundary coordinating entities; governed by a state machine; emits events. UL: `aggregateroot`.
- DomainEntity — Identity‑bearing object whose state evolves via events; owns value collections. UL: `domainentity`.
- EntityId<T> — Value object identity for entities/markers. UL: `entityid_t_`.
- Saga — Aggregate‑of‑aggregates that coordinates participants and maintains causal order (vector clock). UL: `saga`.

## Commands, Queries, Identities
- Command — Imperative request to change state; handled by an AggregateRoot; causes events. UL: `command`.
- Query — Request for information; reads from ReadModel; responds with QueryResponse. UL: `query`.
- CommandEnvelope<C> — Encloses a Command with identity (correlation/causation). UL: `commandenvelope`.
- QueryEnvelope<Q> — Encloses a Query with identity (correlation/causation). UL: `queryenvelope`.
- MessageIdentity — Triplet of correlation_id, causation_id, message_id for causal tracking. UL: `messageidentity`.
- CommandId — Value object id for commands (EntityId<CommandMarker>). UL: `commandid`.
- QueryId — Value object id for queries (EntityId<QueryMarker>). UL: `queryid`.
- EventId — UUID v7 for time‑ordered events. UL: `eventid`.
- AggregateTransactionId — Transaction correlation for multi‑step flows. UL: `aggregatetransactionid`.
- CorrelationId — Identifies a flow; Single or Transaction. UL: `correlationid`.
- CausationId — References the message that caused another. UL: `causationid`.

## Events & Streams
- DomainEvent — Fact that occurred; immutable; drives projections. UL: `domainevent`.
- DomainEventEnvelope<E> — Carries event identity + payload (Either CID | Inline). UL: `domaineventenvelope`.
- EventStream — Conceptual stream of event envelopes; append‑only. UL: `eventstream` (conceptual).
- EventStreamSubscription — Subscription specification (filters); conceptual here. UL: `eventstreamsubscription` (conceptual).

## Projections & Read Path
- Projection — Consumes events and updates read models (trait). UL: `projection`.
- ReadModel — Read‑optimized state built by projections; conceptual in core. UL: `readmodel` (conceptual).
- QueryResponse — Value object result for queries. UL: `queryresponse`.

## Policies & State Machines
- Policy — Pure constraints and guards applied to Aggregates. UL: `policy`.
- StateMachine — Lens term for aggregate behavior; maps to code trait MealyStateMachine. UL: `statemachine` (maps → `formal_domain::MealyStateMachine`).

## Collections & Values
- ValueObject — Immutable, compared by value; descriptors in the domain. UL: `valueobject`.
- ValueCollection — Shape‑agnostic collection (sequence/set/bag) owned by an entity; forms a monoid (⊕, ∅). UL: `valuecollection`.

## Content Addressing (CID)
- DomainCid — Domain‑scoped CID wrapper with content type hint. UL: `domaincid`.
- DomainNode — IPLD‑friendly node envelope (typed metadata + payload CID/codec). UL: `domainnode`.
- DomainPayloadCodec — Codec hint for DomainNode payload (Raw/DagCbor/DagJson). UL: `domainpayloadcodec`.
- PayloadMetadata — Describes the payload (source/version/properties). UL: `payloadmetadata`.
- IPLD.Cid — External IPLD CID reference (external). UL: `ipldcid`.

## Concepts & External Graphs
- Concept — UL concept node used for naming/anchoring. UL: `concept`.
- CIM.Graph — External reference to cim‑graph concept graph (external). UL: `cimgraphconceptgraph`.
- BoundedContext — Context boundary containing aggregates, projections, etc. UL: `boundedcontext`.

## Acknowledgments
- CommandAcknowledgment — Accepted/Rejected status for commands. UL: `commandacknowledgment`.
- QueryAcknowledgment — Accepted/Rejected status for queries. UL: `queryacknowledgment`.

Notes
- Conceptual items exist in the category lens and UL but intentionally have no concrete core type (e.g., ReadModel, EventStream). Infrastructure implements them downstream.
- StateMachine ↔ MealyStateMachine: we document the mapping in the lens to avoid churn in code naming.

Alignment Sources
- Graph: `domain-graph.json:category.objects` and morphisms
- Code: `src/cqrs.rs`, `src/events.rs`, `src/cid.rs`, `src/formal_domain.rs`, `src/projections/`
- UL: `ul-projection.json` (review `ul-projection.diff.json` for pending additions)

