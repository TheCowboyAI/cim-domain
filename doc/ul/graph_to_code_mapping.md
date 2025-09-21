<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Graph ↔ Code Mapping (Lens Naming)

Purpose: document term mappings between the category lens (domain-graph.json) and code identifiers so we keep UL and AST coherent without forcing churn when names differ.

Mappings
- StateMachine (graph) → formal_domain::MealyStateMachine (code)
  - Rationale: aggregates are modeled as Mealy machines; the lens term “StateMachine” stays UL‑friendly.
- ReadModel (graph, conceptual) → Implemented by Projection implementors + storage downstream
  - In core, `Projection` trait updates read models; persistence belongs downstream.
- EventStream (graph, conceptual) → EventStreamSubscription (contract) + stream name (string)
  - Stream mechanics are infrastructure; the lens keeps the concept in UL.
- CommandId / QueryId (graph) → type aliases over `EntityId<CommandMarker|QueryMarker>` (code)
- StateMachine / Policy (graph) → traits `MealyStateMachine` / `Policy` (code)
- DomainEventEnvelope payload (Either(CID|Inline)) → `Either<DomainCid, E>` (code)
- ValueCollection (graph, monoid) → shape‑agnostic collections; proven via tests (Vec concat, BTreeSet union)
- CIM.Graph (external) → referenced graph in `cim-graph` domain; not implemented here
- IPLD.Cid (external) → cid::Cid; used via `DomainNode.payload_cid`

Notes
- Conceptual entries remain in the lens and UL; code stays pure and minimal in core.
- When adding or renaming graph objects, update this mapping and run UL projection in diff‑first mode.

