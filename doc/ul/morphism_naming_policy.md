<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Morphism Naming Policy (DDD + ACT)

Goal: Every morphism name should carry domain meaning, be directional, and reflect a verifiable law. Avoid generic or container verbs like "has_*", "is_a", or unspecified "related_to".

Principles
- Directional semantics: Name describes source → target action or role (e.g., `provides_physical_location_to`).
- Role/Capability verbs: `provides_*`, `acts_as_*_for`, `wraps`, `projects_to`, `updates`, `subscribes_to`, `consumes`, `appends_to`, `governs`, `constrains`, `coordinates`, `addresses_*_by_*`, `identifies_*`, `correlates_with`, `precedes`.
- Concept alignment: For conceptual equivalence across contexts, use `is_partially_equivalent_to` or `refines_concept` instead of generic `is_a`.
- Container context: Use `scopes_*` for BoundedContext containment rather than `contains_*`.

Proposed renames (lens)
- `has_values` → `owns_value_collection` (structural ownership; consider aggregate‑scoped constraints separately)
- `envelope_has_event_id` → `identifies_event`
- `envelope_has_aggregate_id` → `identifies_aggregate`
- `envelope_has_correlation_id` → `correlates_with`
- `envelope_has_causation_id` → `was_caused_by`
- `envelope_has_payload_metadata` → `describes_payload`
- `command_envelope_has_id` / `query_envelope_has_id` → `identified_by`
- `command_envelope_has_identity` / `query_envelope_has_identity` → `carries_identity`
- `identity_has_correlation_id` → `provides_correlation_id`
- `identity_has_causation_id` → `provides_causation_id`
- `identity_has_command_message_id` → `provides_command_message_id`
- `identity_has_query_message_id` → `provides_query_message_id`
- `identity_has_event_message_id` → `provides_event_id`
- `stream_contains_envelope` → `collects_envelope`
- `contains_aggregate|projection|read_model|event_stream|command|query|policy|state_machine|saga` → `scopes_aggregate|scopes_projection|…`
- Concept graph: `is_a` → `refines_concept`; `related_to` → domain‑specific verbs; add `is_partially_equivalent_to` (directional) for nuanced equivalence.

Process
1) Apply renames in `category.morphisms` and update `category.diagrams[*].describes` accordingly (no code changes required).
2) Regenerate UL diff; do not overwrite until review.
3) Update diagrams’ edge labels to match verbs; keep layout readable.
4) Log dialog event and add insight when policy changes.

Notes
- Constraints (ranges/optional/defaults) are aggregate‑scoped invariants/policies; do not encode per relationship.
- Names should be short yet descriptive and testable by reading code/diagrams.
