<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Diagram Plan (Minimal, UL-Aligned)

Goal: replace ad-hoc diagrams with a small, high-value set that reads like English, uses UL-aligned morphism IDs as labels, and keeps validator coverage simple.

Principles
- Use directional verbs (see morphism_naming_policy). Labels derive from morphism `id`.
- One diagram per core flow or view; avoid clutter; left→right flow.
- Keep diagram `describes` lists small and focused. Validator stays green.

Minimal Set (planned)
- event_pipeline_v2
  - describes: handled_by, causes_event, emits_event, wraps_event, references_payload_cid, appended_to_stream, collects_envelope
- identity_envelope_v2
  - describes: identified_by_command_id, encloses_command, command_carries_identity, identified_by_query_id, encloses_query, query_carries_identity, provides_correlation_id, provides_causation_id, provides_event_id, identifies_event, identifies_aggregate, correlates_with, was_caused_by, describes_payload, command_correlates_to_event, query_correlates_to_event, precedes_envelope, acknowledged_by_command, acknowledged_by_query
- read_path_v2
  - describes: subscribes_to_stream, consumes_event, updates_read_model, reads_from, responds_with
- addressing_v2
  - describes: domain_cid_defines_node, uses_payload_codec, payload_is, annotated_by_metadata, defined_by_ipld
- bounded_context_scope_v2
  - describes: scopes_aggregate, scopes_projection, scopes_read_model, scopes_event_stream, scopes_command, scopes_query, scopes_policy, scopes_state_machine, scopes_saga

Implementation Plan
1) Generate DOT via `ul_dot` for each view using `--include` lists above.
2) Convert DOT→SVG (Graphviz) and store under `doc/act/diagrams/`.
3) Add diagram entries back to `domain-graph.json` with the same `describes` lists; run validator.
4) Use `ul_narrative` to produce text alongside the diagrams in docs.

Notes
- Keep string_diagrams.verified=false until the new set is committed and validated.
- Avoid per-relationship constraint blobs; constraints live at Aggregate/Policy.
